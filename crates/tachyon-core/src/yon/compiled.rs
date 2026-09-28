use super::dynamic::SocketCalls;
use super::plan::{Plan, Provider, Route, Scope};
use super::route_calls::RouteCalls;
use std::collections::BTreeMap;
use std::fmt::Write as _;

pub(crate) fn lower(extension: &str, stripped: &str, plan: &Plan) -> Result<String, String> {
    let controller = provider_index(plan, &plan.controller)?;
    let dependencies = dependency_indices(plan)?;
    let generated = match extension {
        "java" | "cs" => managed_container(extension, plan, &dependencies, controller),
        "kt" => kotlin_container(plan, &dependencies, controller),
        "rs" => rust_container(plan, &dependencies, controller),
        "dart" => dart_container(plan, &dependencies, controller),
        "go" => {
            super::go_container::GoContainer::render(stripped, plan, &dependencies, controller)?
        }
        _ => return Err(format!("Unsupported compiled Yon language '{extension}'.")),
    };
    Ok(format!("{stripped}\n{generated}"))
}

pub(crate) fn socket(
    extension: &str,
    stripped: &str,
    plan: &Plan,
    methods: &super::plan::SocketMethods,
) -> Result<String, String> {
    let controller = provider_index(plan, &plan.controller)?;
    let dependencies = dependency_indices(plan)?;
    let generated = match extension {
        "go" => super::go_container::GoContainer::socket(
            stripped,
            plan,
            &dependencies,
            controller,
            methods,
        )?,
        "java" => java_socket(plan, &dependencies, controller, methods),
        "kt" => kotlin_socket(plan, &dependencies, controller, methods),
        "cs" => csharp_socket(plan, &dependencies, controller, methods),
        "rs" => rust_socket(plan, &dependencies, controller, methods),
        "dart" => dart_socket(plan, &dependencies, controller, methods),
        _ => {
            return Err(format!(
                "No Yon socket lowering exists for .{extension} yet"
            ));
        }
    };
    Ok(format!("{stripped}\n{generated}"))
}

fn dart_socket(
    plan: &Plan,
    dependencies: &[Vec<usize>],
    controller: usize,
    methods: &super::plan::SocketMethods,
) -> String {
    let target = &plan.controller;
    let mut generated = dart_providers(plan, dependencies);
    let _ = write!(
        generated,
        "\nfinal class __YonSocketEndpoint implements YonSocketEndpoint {{\n  __YonSocketEndpoint(this.target);\n\n  final {target} target;\n"
    );
    for (event, method, parameters, values) in [
        (
            "open",
            methods.open.as_ref(),
            "SocketSession session",
            ["session", ""],
        ),
        (
            "close",
            methods.close.as_ref(),
            "SocketSession session",
            ["session", ""],
        ),
        (
            "error",
            methods.error.as_ref(),
            "SocketSession session, Object error",
            ["session", "error"],
        ),
    ] {
        let body = method.map_or_else(
            || String::from("null"),
            |method| {
                format!(
                    "target.{}({})",
                    method.name,
                    SocketCalls::arguments(method, &values)
                )
            },
        );
        let _ = write!(
            generated,
            "\n  @override\n  dynamic {event}({parameters}) => {body};\n"
        );
    }
    let message = SocketCalls::message_type(plan)
        .and_then(|kind| crate::route_types::TypeText::shape("dart", kind).ok())
        .map_or_else(
            || String::from("value"),
            |shape| crate::route_types::DartValues::read(&shape, "value"),
        );
    let _ = write!(
        generated,
        "\n  @override\n  dynamic message(SocketSession session, Object? value) => target.{}(session, {message});\n}}\n\nfinal class Handler {{\n  Object? GET(__YonFrame request) {{\n    final container = __YonContainer();\n    return YonSocket.run(__YonSocketEndpoint(container.resolve_{controller}()), request, __yon_json);\n  }}\n}}\n",
        methods.message
    );
    generated
}

fn rust_socket(
    plan: &Plan,
    dependencies: &[Vec<usize>],
    controller: usize,
    methods: &super::plan::SocketMethods,
) -> String {
    let target = &plan.controller;
    let mut generated = rust_providers(plan, dependencies);
    let _ = write!(
        generated,
        "struct __YonSocketEndpoint {{\n    target: std::sync::Arc<{target}>,\n}}\n\n#[allow(unused_variables)]\nimpl YonSocketEndpoint for __YonSocketEndpoint {{\n"
    );
    for (event, method, parameters, values) in [
        (
            "open",
            methods.open.as_ref(),
            "session: &SocketSession",
            ["session", ""],
        ),
        (
            "close",
            methods.close.as_ref(),
            "session: &SocketSession",
            ["session", ""],
        ),
        (
            "error",
            methods.error.as_ref(),
            "session: &SocketSession, error: &str",
            ["session", "error"],
        ),
    ] {
        let body = method.map_or_else(
            || String::from("None"),
            |method| {
                format!(
                    "YonSocketResult::failure(self.target.{}({}))",
                    method.name,
                    SocketCalls::arguments(method, &values)
                )
            },
        );
        let _ = writeln!(
            generated,
            "    fn {event}(&self, {parameters}) -> Option<String> {{\n        {body}\n    }}"
        );
    }
    let kind = SocketCalls::message_type(plan).unwrap_or("Json");
    let _ = write!(
        generated,
        "    fn message(&self, session: &SocketSession, value: &Json) -> Option<String> {{\n        YonSocketResult::failure(self.target.{message}(session, <{kind} as YonRead>::read(value)))\n    }}\n}}\n\nstruct Handler;\n#[allow(non_snake_case)]\nimpl Handler {{\n    fn GET(request: &__YonFrame) -> __YonAnswer {{\n        let endpoint = __YonSocketEndpoint {{ target: __YonContainer::default().resolve_{controller}() }};\n        YonSocket::run(&endpoint, request)\n    }}\n}}\n",
        message = methods.message
    );
    generated
}

fn csharp_socket(
    plan: &Plan,
    dependencies: &[Vec<usize>],
    controller: usize,
    methods: &super::plan::SocketMethods,
) -> String {
    let target = &plan.controller;
    let mut generated = managed_providers("cs", plan, dependencies);
    let _ = write!(
        generated,
        "sealed class __YonSocketEndpoint : IYonSocketEndpoint\n{{\n    private readonly {target} target;\n\n    internal __YonSocketEndpoint({target} target)\n    {{\n        this.target = target;\n    }}\n\n"
    );
    for (event, method, parameters, values) in [
        (
            "Open",
            methods.open.as_ref(),
            "SocketSession session",
            ["session", ""],
        ),
        (
            "Close",
            methods.close.as_ref(),
            "SocketSession session",
            ["session", ""],
        ),
        (
            "Error",
            methods.error.as_ref(),
            "SocketSession session, Exception error",
            ["session", "error"],
        ),
    ] {
        let body = method.map_or_else(String::new, |method| {
            format!(
                " YonSocket.Call(() => target.{}({})); ",
                method.name,
                SocketCalls::arguments(method, &values)
            )
        });
        let _ = writeln!(
            generated,
            "    public void {event}({parameters}) {{{body}}}"
        );
    }
    let kind = SocketCalls::message_type(plan).unwrap_or("object");
    let _ = write!(
        generated,
        "    public void Message(SocketSession session, string data)\n    {{\n        var message = YonSocket.Read<{kind}>(data);\n        YonSocket.Call(() => target.{message}(session, message));\n    }}\n}}\n\nsealed class Handler\n{{\n    public static object GET(__YonFrame request)\n    {{\n        var container = new __YonContainer();\n        return YonSocket.Run(new __YonSocketEndpoint(container.resolve_{controller}()), request);\n    }}\n}}\n",
        message = methods.message
    );
    generated
}

fn kotlin_socket(
    plan: &Plan,
    dependencies: &[Vec<usize>],
    controller: usize,
    methods: &super::plan::SocketMethods,
) -> String {
    let target = &plan.controller;
    let mut generated = kotlin_providers(plan, dependencies);
    let _ = writeln!(
        generated,
        "private class __YonSocketEndpoint(private val target: {target}) : YonSocketEndpoint {{"
    );
    for (event, method, parameters, values) in [
        (
            "open",
            methods.open.as_ref(),
            "session: SocketSession",
            ["session", ""],
        ),
        (
            "close",
            methods.close.as_ref(),
            "session: SocketSession",
            ["session", ""],
        ),
        (
            "error",
            methods.error.as_ref(),
            "session: SocketSession, error: Exception",
            ["session", "error"],
        ),
    ] {
        let body = method.map_or_else(String::new, |method| {
            format!(
                " target.{}({}) ",
                method.name,
                SocketCalls::arguments(method, &values)
            )
        });
        let _ = writeln!(
            generated,
            "    override fun {event}({parameters}) {{{body}}}"
        );
    }
    let kind = SocketCalls::message_type(plan).unwrap_or("Any?");
    let _ = write!(
        generated,
        "    override fun message(session: SocketSession, value: Any?) {{\n        target.{message}(session, YonModels.argument({target}::class.java, \"{message}\", 2, 1, value) as {kind})\n    }}\n}}\n\n@Suppress(\"UNUSED_PARAMETER\")\nobject Handler {{\n    fun GET(request: __YonFrame): Any? {{ val container = __YonContainer(); return YonSocket.run(__YonSocketEndpoint(container.resolve_{controller}()), request) }}\n}}\n",
        message = methods.message
    );
    generated
}

fn java_socket(
    plan: &Plan,
    dependencies: &[Vec<usize>],
    controller: usize,
    methods: &super::plan::SocketMethods,
) -> String {
    let target = &plan.controller;
    let mut generated = managed_providers("java", plan, dependencies);
    let _ = write!(
        generated,
        "final class __YonSocketEndpoint implements YonSocketEndpoint {{\n    private final {target} target;\n\n    __YonSocketEndpoint({target} target) {{ this.target = target; }}\n\n"
    );
    for (event, method, parameters, values) in [
        (
            "open",
            methods.open.as_ref(),
            "SocketSession session",
            ["session", ""],
        ),
        (
            "close",
            methods.close.as_ref(),
            "SocketSession session",
            ["session", ""],
        ),
        (
            "error",
            methods.error.as_ref(),
            "SocketSession session, Exception error",
            ["session", "error"],
        ),
    ] {
        let body = method.map_or_else(String::new, |method| {
            format!(
                " target.{}({}); ",
                method.name,
                SocketCalls::arguments(method, &values)
            )
        });
        let _ = writeln!(
            generated,
            "    public void {event}({parameters}) throws Exception {{{body}}}"
        );
    }
    let kind = SocketCalls::message_type(plan).unwrap_or("Object");
    let _ = write!(
        generated,
        "    public void message(SocketSession session, Object value) throws Exception {{\n        target.{message}(session, ({kind}) YonModels.argument({target}.class, \"{message}\", 2, 1, value));\n    }}\n}}\n\nfinal class Handler {{\n    public static Object GET(__YonFrame request) throws Exception {{\n        var container = new __YonContainer();\n        return YonSocket.run(new __YonSocketEndpoint(container.resolve_{controller}()), request);\n    }}\n}}\n",
        message = methods.message
    );
    generated
}

pub(crate) fn gateway(
    extension: &str,
    stripped: &str,
    plan: &Plan,
    methods: &BTreeMap<String, usize>,
) -> Result<String, String> {
    if !matches!(extension, "java" | "kt" | "cs" | "rs" | "dart" | "go") {
        return Err(format!("No Yon gateway unit exists for .{extension} yet."));
    }
    let target = provider_index(plan, &plan.controller)?;
    let dependencies = dependency_indices(plan)?;
    let mut calls = Vec::new();
    for (method, arity) in methods {
        calls.push(gateway_call(
            extension,
            stripped,
            &plan.controller,
            method,
            *arity,
        )?);
    }
    let generated = match extension {
        "java" => {
            let providers = managed_providers("java", plan, &dependencies);
            format!("{providers}{}", java_gateway_handler(target, &calls))
        }
        "kt" => {
            let providers = kotlin_providers(plan, &dependencies);
            format!("{providers}{}", kotlin_gateway_handler(target, &calls))
        }
        "cs" => {
            let providers = managed_providers("cs", plan, &dependencies);
            let runtime = crate::behavior::gateway_unit_runtime("cs", stripped).unwrap_or_default();
            format!(
                "{providers}{}{runtime}",
                csharp_gateway_handler(target, &calls)
            )
        }
        "rs" => {
            let providers = rust_providers(plan, &dependencies);
            let runtime = crate::behavior::gateway_unit_runtime("rs", stripped).unwrap_or_default();
            format!(
                "{providers}{}{runtime}",
                rust_gateway_handler(target, &calls)
            )
        }
        "go" => {
            let providers = super::go_container::GoContainer::providers(plan, &dependencies);
            let runtime = crate::behavior::gateway_unit_runtime("go", stripped).unwrap_or_default();
            format!(
                "{}container\npackage main\n{providers}{}{runtime}",
                crate::go_source::GoFile::MARKER,
                go_gateway_handler(target, &calls)
            )
        }
        _ => {
            let providers = dart_providers(plan, &dependencies);
            format!("{providers}{}", dart_gateway_handler(target, &calls))
        }
    };
    Ok(format!("{stripped}\n{generated}"))
}

struct GatewayCall {
    method: String,
    arity: usize,
    result_type: String,
}

fn gateway_call(
    extension: &str,
    stripped: &str,
    class: &str,
    method: &str,
    arity: usize,
) -> Result<GatewayCall, String> {
    let path = std::path::PathBuf::from(format!("{class}.{extension}"));
    let signature =
        crate::behavior::gateway_target_signature(&path, stripped, class, method, arity)
            .map_err(|error| format!("Gateway target {class}.{method} cannot be read: {error}"))?;
    let Some(signature) = signature else {
        return Err(format!("Gateway target {class}.{method} is not declared."));
    };
    Ok(GatewayCall {
        method: method.to_owned(),
        arity: signature.parameter_types.len(),
        result_type: signature.result_type,
    })
}

fn unit_argument_list(arity: usize) -> String {
    let mut arguments = Vec::new();
    for index in 0..arity {
        arguments.push(format!("arguments.get({index})"));
    }
    arguments.join(", ")
}

fn java_gateway_handler(target: usize, calls: &[GatewayCall]) -> String {
    let mut generated = String::from(
        "final class Handler {\n    public static Object POST(__YonFrame request) throws Exception {\n        java.util.Map<?, ?> call = (java.util.Map<?, ?>) Json.parse(request.body());\n        java.util.List<?> arguments = (java.util.List<?>) call.get(\"arguments\");\n        Object method = call.get(\"method\");\n        java.util.Map<String, Object> answer = new java.util.HashMap<>();\n",
    );
    let _ = writeln!(
        generated,
        "        var target = new __YonContainer().resolve_{target}();"
    );
    for call in calls {
        let _ = writeln!(
            generated,
            "        if ({}.equals(method)) {{\n            answer.put(\"result\", target.{}({}));\n            return answer;\n        }}",
            quoted(&call.method),
            call.method,
            unit_argument_list(call.arity)
        );
    }
    generated.push_str(
        "        throw new IllegalArgumentException(\"Gateway method is not declared.\");\n    }\n}\n",
    );
    generated
}

fn kotlin_gateway_handler(target: usize, calls: &[GatewayCall]) -> String {
    let mut generated = String::from(
        "object Handler {\n    fun POST(request: __YonFrame): Any? {\n        val call = Json.parse(request.body()) as Map<*, *>\n        val arguments = call[\"arguments\"] as List<*>\n",
    );
    let _ = writeln!(
        generated,
        "        val target = __YonContainer().resolve_{target}()"
    );
    generated.push_str("        val result: Any? = when (call[\"method\"]) {\n");
    for call in calls {
        let _ = writeln!(
            generated,
            "            {} -> target.{}({})",
            quoted(&call.method),
            call.method,
            unit_argument_list(call.arity)
        );
    }
    generated.push_str(
        "            else -> throw IllegalArgumentException(\"Gateway method is not declared.\")\n        }\n        return hashMapOf(\"result\" to result)\n    }\n}\n",
    );
    generated
}

fn csharp_gateway_handler(target: usize, calls: &[GatewayCall]) -> String {
    let mut generated = String::from(
        "sealed class Handler {\n    public static object POST(__YonFrame request) {\n        var call = System.Text.Json.Nodes.JsonNode.Parse(request.Body()).AsObject();\n        var arguments = call[\"arguments\"].AsArray();\n        var method = call[\"method\"].GetValue<string>();\n",
    );
    let _ = writeln!(
        generated,
        "        var target = new __YonContainer().resolve_{target}();"
    );
    for call in calls {
        let mut arguments = Vec::new();
        for index in 0..call.arity {
            arguments.push(format!("GatewayArgument(arguments, {index})"));
        }
        let _ = writeln!(
            generated,
            "        if (method == {}) return GatewayAnswer(target.{}({}));",
            quoted(&call.method),
            call.method,
            arguments.join(", ")
        );
    }
    generated.push_str(
        "        throw new System.ArgumentException(\"Gateway method is not declared.\");\n    }\n    static BehaviorJSON GatewayArgument(System.Text.Json.Nodes.JsonArray arguments, int index) => BehaviorJSON.Parse(arguments[index]?.ToJsonString() ?? \"null\");\n    static __YonAnswer GatewayAnswer(BehaviorJSON result) => __YonAnswer.Json(\"{\\\"result\\\":\" + BehaviorJSON.From(result).ToString() + \"}\");\n}\n",
    );
    generated
}

fn rust_gateway_handler(target: usize, calls: &[GatewayCall]) -> String {
    let mut generated = String::from(
        "struct Handler;\n#[allow(non_snake_case)]\nimpl Handler {\n    fn POST(request: &__YonFrame) -> __YonAnswer {\n        let Ok(call) = request.json() else {\n            return __YonAnswer::empty(400);\n        };\n        let arguments: Vec<BehaviorValue> = call.get(\"arguments\").items().iter().cloned().map(BehaviorValue::from).collect();\n",
    );
    let _ = writeln!(
        generated,
        "        let target = __YonContainer::default().resolve_{target}();"
    );
    generated.push_str("        let result = match call.get(\"method\").text().as_str() {\n");
    for call in calls {
        let mut arguments = Vec::new();
        for index in 0..call.arity {
            arguments.push(format!("arguments[{index}].clone()"));
        }
        let invocation = format!("target.{}({})", call.method, arguments.join(", "));
        let returns_value = call.result_type.replace(' ', "") == "BehaviorValue";
        let result = if returns_value {
            format!("Ok({invocation})")
        } else {
            invocation
        };
        let _ = writeln!(
            generated,
            "            {} => {result},",
            quoted(&call.method)
        );
    }
    generated.push_str(
        "            _ => return __YonAnswer::empty(400),\n        };\n        match result.and_then(|value| value.to_json(1_048_576)) {\n            Ok(value) => __YonAnswer::json(format!(\"{{\\\"result\\\":{value}}}\")),\n            Err(_) => __YonAnswer::empty(500),\n        }\n    }\n}\n",
    );
    generated
}

fn dart_gateway_handler(target: usize, calls: &[GatewayCall]) -> String {
    let mut generated = String::from(
        "\nfinal class Handler {\n  Future<Object?> POST(__YonFrame request) async {\n    final call = yon_convert.jsonDecode(request.body()) as Map;\n    final arguments = call['arguments'] as List;\n",
    );
    let _ = writeln!(
        generated,
        "    final target = __YonContainer().resolve_{target}();"
    );
    generated.push_str("    final Object? result;\n    switch (call['method']) {\n");
    for call in calls {
        let mut arguments = Vec::new();
        for index in 0..call.arity {
            arguments.push(format!("arguments[{index}]"));
        }
        let _ = writeln!(
            generated,
            "      case {}:\n        result = await target.{}({});",
            quoted(&call.method).replace('$', "\\$"),
            call.method,
            arguments.join(", ")
        );
    }
    generated.push_str(
        "      default:\n        throw ArgumentError('Gateway method is not declared.');\n    }\n    return {'result': result};\n  }\n}\n",
    );
    generated
}

fn go_gateway_handler(target: usize, calls: &[GatewayCall]) -> String {
    let mut generated = String::from(
        "\ntype Handler struct{}\n\nfunc (Handler) POST(request *yonRequest) *yonResponse {\n\tcall, err := behaviorParseJSON(request.Body())\n\tfields, isObject := call.(map[string]any)\n\tif err != nil || !isObject {\n\t\treturn yonEmpty(400)\n\t}\n",
    );
    let takes_arguments = calls.iter().any(|call| call.arity > 0);
    if takes_arguments {
        generated.push_str("\targuments, _ := fields[\"arguments\"].([]any)\n");
    }
    generated.push_str("\tmethod, _ := fields[\"method\"].(string)\n");
    let _ = writeln!(
        generated,
        "\ttarget := (&__YonContainer{{}}).resolve_{target}()"
    );
    generated.push_str("\tvar result BehaviorValue\n\tswitch method {\n");
    for call in calls {
        go_gateway_case(&mut generated, call);
    }
    generated.push_str(
        "\tdefault:\n\t\treturn yonEmpty(400)\n\t}\n\tif err != nil {\n\t\treturn yonEmpty(500)\n\t}\n\ttext, err := behaviorEncode(result, 1048576)\n\tif err != nil {\n\t\treturn yonEmpty(500)\n\t}\n\treturn yonJSON(`{\"result\":` + text + `}`)\n}\n",
    );
    generated
}

fn go_gateway_case(generated: &mut String, call: &GatewayCall) {
    let mut arguments = Vec::new();
    for index in 0..call.arity {
        arguments.push(format!("arguments[{index}]"));
    }
    let exported_name = crate::go_source::upper_first(&call.method);
    let returns_only_a_value = call.result_type.replace(' ', "") == "BehaviorValue";
    let results = if returns_only_a_value {
        "result"
    } else {
        "result, err"
    };
    let _ = writeln!(
        generated,
        "\tcase {}:\n\t\t{results} = target.{exported_name}({})",
        quoted(&call.method),
        arguments.join(", ")
    );
}

fn quoted(text: &str) -> String {
    serde_json::Value::String(text.to_owned()).to_string()
}

fn dependency_indices(plan: &Plan) -> Result<Vec<Vec<usize>>, String> {
    let mut dependencies = Vec::with_capacity(plan.providers.len());
    for provider in &plan.providers {
        let mut indices = Vec::with_capacity(provider.dependencies.len());
        for name in &provider.dependencies {
            indices.push(provider_index(plan, name)?);
        }
        dependencies.push(indices);
    }
    Ok(dependencies)
}

fn provider_index(plan: &Plan, name: &str) -> Result<usize, String> {
    plan.providers
        .iter()
        .position(|provider| provider.name == name)
        .ok_or_else(|| format!("No provider is declared for dependency '{name}'."))
}

fn cached(provider: &Provider) -> bool {
    provider.scope != Scope::Transient
}

fn resolver_arguments(dependencies: &[usize]) -> String {
    dependencies
        .iter()
        .map(|dependency| format!("resolve_{dependency}()"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn managed_container(
    extension: &str,
    plan: &Plan,
    dependencies: &[Vec<usize>],
    controller: usize,
) -> String {
    let java = extension == "java";
    let declaration = if java { "final" } else { "sealed" };
    let mut generated = managed_providers(extension, plan, dependencies);
    let _ = writeln!(generated, "{declaration} class Handler {{");
    for route in &plan.routes {
        managed_route(&mut generated, plan, route, controller, java);
    }
    generated.push_str("}\n");
    generated
}

fn managed_providers(extension: &str, plan: &Plan, dependencies: &[Vec<usize>]) -> String {
    let java = extension == "java";
    let declaration = if java { "final" } else { "sealed" };
    let mut generated = format!("\n{declaration} class __YonContainer {{\n");
    for (index, provider) in plan.providers.iter().enumerate() {
        managed_resolver(&mut generated, provider, index, &dependencies[index], java);
    }
    generated.push_str("}\n");
    generated
}

fn managed_resolver(
    generated: &mut String,
    provider: &Provider,
    index: usize,
    dependencies: &[usize],
    java: bool,
) {
    let throws = if java { " throws Exception" } else { "" };
    let visibility = if java { "" } else { "internal " };
    if cached(provider) {
        let _ = writeln!(generated, "    private {} p{index};", provider.name);
    }
    let arguments = resolver_arguments(dependencies);
    let _ = writeln!(
        generated,
        "    {visibility}{} resolve_{index}(){throws} {{",
        provider.name
    );
    if cached(provider) {
        let _ = writeln!(
            generated,
            "        if (p{index} == null) p{index} = new {}({arguments});\n        return p{index};",
            provider.name
        );
    } else {
        let _ = writeln!(
            generated,
            "        return new {}({arguments});",
            provider.name
        );
    }
    generated.push_str("    }\n");
}

fn managed_route(
    generated: &mut String,
    plan: &Plan,
    route: &Route,
    controller: usize,
    java: bool,
) {
    let argument = match (route.takes_request, route.dispatcher, java) {
        (false, _, _) => "",
        (true, true, _) => "request",
        (true, false, true) => "Yon.exchange(request)",
        (true, false, false) => "YonHttp.Request(request)",
    };
    let result_type = if java { "Object" } else { "object" };
    let throws = if java { " throws Exception" } else { "" };
    let errors = RouteCalls::error_owners(plan)
        .iter()
        .map(|(index, _)| format!("container.resolve_{index}()"))
        .collect::<Vec<_>>();
    let invocation = RouteCalls::invocation_errors(
        route,
        &format!("container.resolve_{controller}()"),
        ".",
        argument,
        &errors,
    );
    let _ = writeln!(
        generated,
        "    public static {result_type} {}(__YonFrame request){throws} {{\n        var container = new __YonContainer();\n        return {invocation};\n    }}",
        route.verb
    );
}

fn dart_container(plan: &Plan, dependencies: &[Vec<usize>], controller: usize) -> String {
    let mut generated = dart_providers(plan, dependencies);
    generated.push_str("\nfinal class Handler {\n");
    for route in &plan.routes {
        let argument = match (route.takes_request, route.dispatcher) {
            (false, _) => "",
            (true, true) => "request",
            (true, false) => "_yonShelfRequest(request)",
        };
        let errors = RouteCalls::error_owners(plan)
            .iter()
            .map(|(index, _)| format!("container.resolve_{index}()"))
            .collect::<Vec<_>>();
        let invocation = RouteCalls::invocation_errors(
            route,
            &format!("container.resolve_{controller}()"),
            ".",
            argument,
            &errors,
        );
        let _ = writeln!(
            generated,
            "  Object? {}(__YonFrame request) {{ final container = __YonContainer(); return {invocation}; }}",
            route.verb
        );
    }
    generated.push_str("}\n");
    generated
}

fn dart_providers(plan: &Plan, dependencies: &[Vec<usize>]) -> String {
    let mut generated = String::from("\nfinal class __YonContainer {\n");
    for (index, provider) in plan.providers.iter().enumerate() {
        let arguments = resolver_arguments(&dependencies[index]);
        if cached(provider) {
            let _ = writeln!(
                generated,
                "  {}? _p{index};\n  {} resolve_{index}() => _p{index} ??= {}({arguments});",
                provider.name, provider.name, provider.name
            );
        } else {
            let _ = writeln!(
                generated,
                "  {} resolve_{index}() => {}({arguments});",
                provider.name, provider.name
            );
        }
    }
    generated.push_str("}\n");
    generated
}

fn kotlin_container(plan: &Plan, dependencies: &[Vec<usize>], controller: usize) -> String {
    let mut generated = kotlin_providers(plan, dependencies);
    generated.push_str("@Suppress(\"UNUSED_PARAMETER\")\nobject Handler {\n");
    for route in &plan.routes {
        let argument = match (route.takes_request, route.dispatcher) {
            (false, _) => "",
            (true, true) => "request",
            (true, false) => "Yon.exchange(request)",
        };
        let result_type = "Any?";
        let errors = RouteCalls::error_owners(plan)
            .iter()
            .map(|(index, _)| format!("container.resolve_{index}()"))
            .collect::<Vec<_>>();
        let invocation = RouteCalls::invocation_errors(
            route,
            &format!("container.resolve_{controller}()"),
            ".",
            argument,
            &errors,
        );
        let _ = writeln!(
            generated,
            "    fun {}(request: __YonFrame): {result_type} {{ val container = __YonContainer(); return {invocation} }}",
            route.verb
        );
    }
    generated.push_str("}\n");
    generated
}

fn kotlin_providers(plan: &Plan, dependencies: &[Vec<usize>]) -> String {
    let mut generated = String::from("\nprivate class __YonContainer {\n");
    for (index, provider) in plan.providers.iter().enumerate() {
        let arguments = resolver_arguments(&dependencies[index]);
        if cached(provider) {
            let _ = writeln!(
                generated,
                "    private val p{index}: {} by lazy {{ {}({arguments}) }}\n    fun resolve_{index}(): {} = p{index}",
                provider.name, provider.name, provider.name
            );
        } else {
            let _ = writeln!(
                generated,
                "    fun resolve_{index}(): {} = {}({arguments})",
                provider.name, provider.name
            );
        }
    }
    generated.push_str("}\n");
    generated
}

fn rust_container(plan: &Plan, dependencies: &[Vec<usize>], controller: usize) -> String {
    let mut generated = rust_providers(plan, dependencies);
    generated.push_str("struct Handler;\n#[allow(non_snake_case)]\nimpl Handler {\n");
    for route in &plan.routes {
        let argument = match (route.takes_request, route.dispatcher) {
            (false, _) => "",
            (true, true) => "request",
            (true, false) => "&yon_native_request(request)",
        };
        let parameter = if route.takes_request {
            "request"
        } else {
            "_request"
        };
        let errors = rust_error_handlers(plan);
        if errors.is_empty() {
            let invocation = RouteCalls::invocation(
                route,
                &RouteCalls::borrowed(
                    route,
                    &format!("__YonContainer::default().resolve_{controller}()"),
                ),
                argument,
            );
            let _ = writeln!(
                generated,
                "    fn {}({parameter}: &__YonFrame) -> __YonAnswer {{\n        yon_response({invocation})\n    }}",
                route.verb
            );
            continue;
        }
        let controller_value =
            RouteCalls::borrowed(route, &format!("container.resolve_{controller}()"));
        let invocation =
            RouteCalls::invocation_errors(route, &controller_value, ".", argument, &errors);
        let _ = writeln!(
            generated,
            "    fn {}({parameter}: &__YonFrame) -> __YonAnswer {{\n        let mut container = __YonContainer::default();\n        yon_response({invocation})\n    }}",
            route.verb
        );
    }
    generated.push_str("}\n");
    generated
}

fn rust_error_handlers(plan: &Plan) -> Vec<String> {
    let mut handlers = Vec::new();
    for (index, _) in RouteCalls::error_owners(plan) {
        handlers.push(format!("&container.resolve_{index}()"));
    }
    handlers
}

fn rust_providers(plan: &Plan, dependencies: &[Vec<usize>]) -> String {
    let mut generated = String::from("\n#[derive(Default)]\nstruct __YonContainer {\n");
    for (index, provider) in plan.providers.iter().enumerate() {
        if cached(provider) {
            let _ = writeln!(
                generated,
                "    p{index}: Option<std::sync::Arc<{}>>,",
                provider.name
            );
        }
    }
    generated.push_str("}\nimpl __YonContainer {\n");
    for (index, provider) in plan.providers.iter().enumerate() {
        rust_resolver(&mut generated, index, provider, &dependencies[index]);
    }
    generated.push_str("}\n");
    generated
}

fn rust_resolver(
    generated: &mut String,
    index: usize,
    provider: &Provider,
    dependencies: &[usize],
) {
    let _ = writeln!(
        generated,
        "    fn resolve_{index}(&mut self) -> std::sync::Arc<{}> {{",
        provider.name
    );
    if cached(provider) {
        let _ = writeln!(
            generated,
            "        if let Some(value) = &self.p{index} {{ return std::sync::Arc::clone(value); }}"
        );
    }
    for (position, dependency) in dependencies.iter().enumerate() {
        let _ = writeln!(
            generated,
            "        let dependency_{position} = self.resolve_{dependency}();"
        );
    }
    let arguments = (0..dependencies.len())
        .map(|position| format!("dependency_{position}"))
        .collect::<Vec<_>>()
        .join(", ");
    let _ = writeln!(
        generated,
        "        let value = std::sync::Arc::new({}::new({arguments}));",
        provider.name
    );
    if cached(provider) {
        let _ = writeln!(
            generated,
            "        self.p{index} = Some(std::sync::Arc::clone(&value));"
        );
    }
    generated.push_str("        value\n    }\n");
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::super::plan::{Role, Route};
    use super::*;

    fn provider(name: &str, role: Role, scope: Scope, dependencies: &[&str]) -> Provider {
        Provider {
            name: name.into(),
            role,
            scope,
            dependencies: dependencies.iter().map(|name| (*name).into()).collect(),
            injections: Vec::new(),
        }
    }

    fn plan() -> Plan {
        Plan {
            errors: Vec::new(),
            providers: vec![
                provider(
                    "Products",
                    Role::Controller,
                    Scope::Request,
                    &["Store", "Fresh"],
                ),
                provider("Store", Role::Service, Scope::Singleton, &[]),
                provider("Fresh", Role::Service, Scope::Transient, &[]),
            ],
            controller: "Products".into(),
            routes: vec![
                Route {
                    verb: "GET".into(),
                    method: "list".into(),
                    streaming: false,
                    takes_request: true,
                    ..Route::default()
                },
                Route {
                    verb: "POST".into(),
                    method: "create".into(),
                    streaming: false,
                    takes_request: false,
                    ..Route::default()
                },
            ],
        }
    }

    #[test]
    fn compiled_frontends_preserve_source_and_lower_named_instance_methods() {
        for extension in ["java", "cs", "kt", "rs"] {
            let generated = lower(extension, "// authored source", &plan()).expect("valid plan");
            assert!(generated.starts_with("// authored source\n"));
            let listed = match extension {
                "java" | "kt" => "list(Yon.exchange(request))",
                "cs" => "list(YonHttp.Request(request))",
                "rs" => "list(&yon_native_request(request))",
                _ => "list(request)",
            };
            assert!(generated.contains(listed), "{extension}");
            assert!(generated.contains("create()"));
            assert!(!generated.contains("create(request)"));
            assert!(!generated.contains("getDeclaredConstructor"));
            assert!(!generated.contains("Activator.CreateInstance"));
        }
    }

    #[test]
    fn managed_wrappers_type_check_single_responses_without_erasing_stream_types() {
        let mut plan = plan();
        plan.routes[1].streaming = true;
        let csharp = lower("cs", "", &plan).expect("C# plan");
        assert!(csharp.contains("public static object GET(__YonFrame request)"));
        assert!(csharp.contains("public static object POST(__YonFrame request)"));
        let kotlin = lower("kt", "", &plan).expect("Kotlin plan");
        assert!(kotlin.contains("fun GET(request: __YonFrame): Any? {"));
        assert!(kotlin.contains("fun POST(request: __YonFrame): Any? {"));
    }

    #[test]
    fn unresolved_dependencies_and_unsupported_languages_fail_before_emission() {
        let mut invalid = plan();
        invalid.providers[0].dependencies.push("Missing".into());
        let missing = lower("java", "", &invalid).expect_err("missing provider");
        assert!(missing.contains("Missing"));
        let unsupported = lower("swift", "", &plan()).expect_err("unsupported language");
        assert!(unsupported.contains("swift"));
        invalid.controller = "Absent".into();
        let absent = lower("rs", "", &invalid).expect_err("missing controller");
        assert!(absent.contains("Absent"));
    }

    fn gateway_plan() -> Plan {
        let mut plan = plan();
        plan.controller = "Store".into();
        plan.routes.clear();
        plan
    }

    fn gateway_methods() -> BTreeMap<String, usize> {
        BTreeMap::from([("summarize".into(), 2), ("count".into(), 0)])
    }

    #[test]
    fn a_java_gateway_unit_calls_each_declared_method_with_its_arguments() {
        let source = "class Store {\n    Object summarize(Object mode, Object detail) { return mode; }\n    Object count() { return 3; }\n}\n";
        let unit = gateway("java", source, &gateway_plan(), &gateway_methods()).expect("Java unit");
        assert!(unit.starts_with(source));
        assert!(unit.contains("final class __YonContainer {"));
        assert!(unit.contains("public static Object POST(__YonFrame request) throws Exception {"));
        assert!(unit.contains("var target = new __YonContainer().resolve_1();"));
        assert!(unit.contains(
            "if (\"summarize\".equals(method)) {\n            answer.put(\"result\", target.summarize(arguments.get(0), arguments.get(1)));"
        ));
        assert!(unit.contains("answer.put(\"result\", target.count());"));
        assert!(
            unit.contains(
                "throw new IllegalArgumentException(\"Gateway method is not declared.\");"
            )
        );
    }

    #[test]
    fn a_kotlin_gateway_unit_calls_expression_bodied_methods() {
        let source = "class Store {\n    fun summarize(mode: Any?, detail: Any?): Any? = mode\n    fun count(): Any? { return 3 }\n}\n";
        let unit = gateway("kt", source, &gateway_plan(), &gateway_methods()).expect("Kotlin unit");
        assert!(unit.contains("private class __YonContainer {"));
        assert!(unit.contains("fun POST(request: __YonFrame): Any? {"));
        assert!(unit.contains("val target = __YonContainer().resolve_1()"));
        assert!(
            unit.contains("\"summarize\" -> target.summarize(arguments.get(0), arguments.get(1))")
        );
        assert!(unit.contains("\"count\" -> target.count()"));
        assert!(unit.contains("return hashMapOf(\"result\" to result)"));
    }

    #[test]
    fn a_gateway_unit_refuses_a_missing_method_and_other_languages() {
        let source =
            "class Store {\n    Object summarize(Object mode, Object detail) { return mode; }\n}\n";
        let missing = gateway("java", source, &gateway_plan(), &gateway_methods())
            .expect_err("missing method");
        assert!(missing.contains("Store.count"), "{missing}");
        let unsupported =
            gateway("swift", source, &gateway_plan(), &gateway_methods()).expect_err("Swift unit");
        assert!(unsupported.contains(".swift"), "{unsupported}");
    }

    #[test]
    fn a_csharp_gateway_unit_answers_with_a_result_object() {
        let source = "class Store {\n    public BehaviorJSON summarize(BehaviorJSON mode, BehaviorJSON detail) { return mode; }\n    public BehaviorJSON count() { return BehaviorJSON.From(3); }\n}\n";
        let unit = gateway("cs", source, &gateway_plan(), &gateway_methods()).expect("C# unit");
        assert!(unit.starts_with(source));
        assert!(unit.contains("sealed class __YonContainer {"));
        assert!(unit.contains("public static object POST(__YonFrame request) {"));
        assert!(unit.contains("var target = new __YonContainer().resolve_1();"));
        assert!(unit.contains(
            "if (method == \"summarize\") return GatewayAnswer(target.summarize(GatewayArgument(arguments, 0), GatewayArgument(arguments, 1)));"
        ));
        assert!(unit.contains("if (method == \"count\") return GatewayAnswer(target.count());"));
        assert!(
            unit.contains(
                "throw new System.ArgumentException(\"Gateway method is not declared.\");"
            )
        );
        assert_eq!(
            unit.matches("public sealed class BehaviorJSON {").count(),
            1
        );
    }

    #[test]
    fn a_gateway_unit_adds_the_behavior_runtime_only_when_it_is_missing() {
        let csharp = "class Store {\n    public BehaviorJSON summarize(BehaviorJSON mode, BehaviorJSON detail) { return mode; }\n    public BehaviorJSON count() { return mode; }\n}\npublic sealed class BehaviorJSON {\n}\n";
        let unit = gateway("cs", csharp, &gateway_plan(), &gateway_methods()).expect("C# unit");
        assert_eq!(
            unit.matches("public sealed class BehaviorJSON {").count(),
            1
        );
        let rust = "struct Store;\nimpl Store {\n    fn summarize(&self, mode: BehaviorValue, detail: BehaviorValue) -> BehaviorResult<BehaviorValue> { Ok(mode) }\n    fn count(&self) -> BehaviorValue { BehaviorValue::Null }\n}\npub enum BehaviorValue {\n}\n";
        let unit = gateway("rs", rust, &gateway_plan(), &gateway_methods()).expect("Rust unit");
        assert_eq!(unit.matches("pub enum BehaviorValue {").count(), 1);
        assert!(!unit.contains("impl From<Json> for BehaviorValue"));
    }

    #[test]
    fn a_rust_gateway_unit_wraps_plain_values_in_a_result() {
        let source = "struct Store;\nimpl Store {\n    fn summarize(&self, mode: BehaviorValue, detail: BehaviorValue) -> BehaviorResult<BehaviorValue> { Ok(mode) }\n    fn count(&self) -> BehaviorValue { BehaviorValue::Null }\n}\n";
        let unit = gateway("rs", source, &gateway_plan(), &gateway_methods()).expect("Rust unit");
        assert!(unit.starts_with(source));
        assert!(unit.contains("struct __YonContainer {"));
        assert!(unit.contains("fn POST(request: &__YonFrame) -> __YonAnswer {"));
        assert!(unit.contains("let target = __YonContainer::default().resolve_1();"));
        assert!(unit.contains(
            "\"summarize\" => target.summarize(arguments[0].clone(), arguments[1].clone()),"
        ));
        assert!(unit.contains("\"count\" => Ok(target.count()),"));
        assert!(
            unit.contains(
                "Ok(value) => __YonAnswer::json(format!(\"{{\\\"result\\\":{value}}}\")),"
            )
        );
        assert_eq!(unit.matches("pub enum BehaviorValue {").count(), 1);
        assert!(unit.contains("impl From<Json> for BehaviorValue"));
    }

    #[test]
    fn a_dart_gateway_unit_awaits_block_and_expression_bodies() {
        let source = "class Store {\n  Future<Object?> summarize(Object? mode, Object? detail) async {\n    return mode;\n  }\n  Object? count() => 3;\n}\n";
        let unit = gateway("dart", source, &gateway_plan(), &gateway_methods()).expect("Dart unit");
        assert!(unit.starts_with(source));
        assert!(unit.contains("final class __YonContainer {"));
        assert!(unit.contains("Future<Object?> POST(__YonFrame request) async {"));
        assert!(unit.contains("final target = __YonContainer().resolve_1();"));
        assert!(unit.contains(
            "case \"summarize\":\n        result = await target.summarize(arguments[0], arguments[1]);"
        ));
        assert!(unit.contains("case \"count\":\n        result = await target.count();"));
        assert!(unit.contains("return {'result': result};"));
    }

    #[test]
    fn a_go_gateway_unit_calls_the_exported_methods_through_the_container() {
        let source = "type Store struct{}\n\nfunc (store *Store) Summarize(mode BehaviorValue, detail BehaviorValue) (BehaviorValue, error) {\n\treturn mode, nil\n}\n\nfunc (store Store) Count() BehaviorValue {\n\treturn 3\n}\n";
        let unit = gateway("go", source, &gateway_plan(), &gateway_methods()).expect("Go unit");
        assert!(unit.starts_with(source), "{unit}");
        assert!(unit.contains("\n//tachyon:file container\npackage main\n"));
        assert!(unit.contains("func (container *__YonContainer) resolve_1() *Store {"));
        assert!(unit.contains("func (Handler) POST(request *yonRequest) *yonResponse {"));
        assert!(unit.contains("\targuments, _ := fields[\"arguments\"].([]any)\n"));
        assert!(unit.contains("\ttarget := (&__YonContainer{}).resolve_1()\n"));
        assert!(unit.contains(
            "\tcase \"summarize\":\n\t\tresult, err = target.Summarize(arguments[0], arguments[1])\n"
        ));
        assert!(unit.contains("\tcase \"count\":\n\t\tresult = target.Count()\n"));
        assert!(unit.contains("\treturn yonJSON(`{\"result\":` + text + `}`)\n"));
        assert_eq!(unit.matches("\ntype BehaviorValue = any\n").count(), 1);
        let count_only = BTreeMap::from([("count".into(), 0)]);
        let unit = gateway("go", source, &gateway_plan(), &count_only).expect("Go unit");
        assert!(!unit.contains("arguments, _ :="), "{unit}");
    }

    #[test]
    fn a_go_gateway_unit_keeps_a_behavior_runtime_it_already_has() {
        let source = "type Store struct{}\n\nfunc (store *Store) Summarize(mode BehaviorValue, detail BehaviorValue) (BehaviorValue, error) {\n\treturn mode, nil\n}\n\nfunc (store *Store) Count() BehaviorValue {\n\treturn 3\n}\n\n//tachyon:file behavior\npackage main\n\ntype BehaviorValue = any\n";
        let unit = gateway("go", source, &gateway_plan(), &gateway_methods()).expect("Go unit");
        assert_eq!(unit.matches("\ntype BehaviorValue = any\n").count(), 1);
        assert!(!unit.contains("behaviorModuleID"), "{unit}");
    }

    #[test]
    fn a_go_container_assigns_injected_fields_and_answers_route_results() {
        let mut plan = plan();
        plan.providers[0].injections = vec![
            super::super::plan::Injection {
                field: "store".into(),
                pointer: true,
            },
            super::super::plan::Injection {
                field: "fresh".into(),
                pointer: false,
            },
        ];
        let source = "type Products struct {\n\tstore *Store\n\tfresh Fresh\n}\n\nfunc (products *Products) list(request *http.Request) (any, error) { return nil, nil }\n\nfunc (products *Products) create() {}\n";
        let generated = lower("go", source, &plan).expect("Go plan");
        assert!(generated.starts_with(source), "{generated}");
        assert!(generated.contains("\n//tachyon:file container\npackage main\n"));
        assert!(
            generated.contains("\tp0 *Products\n\tp1 *Store\n}"),
            "{generated}"
        );
        assert!(!generated.contains("p2 *Fresh"), "{generated}");
        assert!(
            generated.contains(
                "\tvalue.store = container.resolve_1()\n\tvalue.fresh = *container.resolve_2()\n"
            ),
            "{generated}"
        );
        assert!(generated.contains("func (Handler) GET(request *yonRequest) (any, error) {\n\treturn (&__YonContainer{}).resolve_0().list(yonNativeRequest(request))\n}"), "{generated}");
        assert!(generated.contains("func (Handler) POST(request *yonRequest) any {\n\t(&__YonContainer{}).resolve_0().create()\n\treturn nil\n}"), "{generated}");
        let missing = lower("go", "type Products struct{}\n", &plan).expect_err("no route");
        assert!(missing.contains("Products.list"), "{missing}");
    }

    #[test]
    fn only_singleton_and_request_providers_have_cache_slots() {
        for extension in ["java", "cs", "kt", "rs"] {
            let generated = lower(extension, "", &plan()).expect("valid plan");
            assert!(generated.contains("p0"));
            assert!(generated.contains("p1"));
            assert!(!generated.contains("p2"));
        }
    }
}
