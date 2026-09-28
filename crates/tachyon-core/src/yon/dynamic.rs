use super::plan::{Plan, Provider, Route, Scope, SocketMethod, SocketMethods};
use std::fmt::Write;

pub(crate) fn lower(extension: &str, stripped: &str, plan: &Plan) -> Result<String, String> {
    match extension {
        "js" | "ts" => javascript(extension, stripped, plan),
        "py" => python(stripped, plan),
        "php" => php(stripped, plan),
        "rb" => RubyLowering::program(stripped, plan),
        _ => Err(format!(
            "No interpreted Yon lowering exists for .{extension}."
        )),
    }
}

pub(crate) fn gateway(
    extension: &str,
    stripped: &str,
    plan: &Plan,
    methods: &std::collections::BTreeMap<String, usize>,
) -> Result<String, String> {
    let target = provider_index(plan, &plan.controller)?;
    let mut output = match extension {
        "js" | "ts" => javascript_providers(extension, stripped, plan)?,
        "py" => python_providers(stripped, plan)?,
        "php" => php_providers(stripped, plan)?,
        "rb" => RubyLowering::providers(stripped, plan)?,
        _ => {
            return Err(format!(
                "No interpreted Yon gateway unit exists for .{extension}."
            ));
        }
    };
    let names: Vec<String> = methods.keys().cloned().collect();
    match extension {
        "js" | "ts" => javascript_gateway(&mut output, extension == "ts", target, &names),
        "py" => python_gateway(&mut output, target, &names),
        "rb" => RubyLowering::gateway(&mut output, target, &names),
        _ => php_gateway(&mut output, target, &names),
    }
    Ok(output)
}

fn javascript_providers(extension: &str, stripped: &str, plan: &Plan) -> Result<String, String> {
    let mut output = format!("{stripped}\n");
    for (index, provider) in plan.providers.iter().enumerate() {
        javascript_provider(&mut output, plan, provider, index, extension == "ts")?;
    }
    Ok(output)
}

fn javascript_gateway(output: &mut String, typed: bool, target: usize, methods: &[String]) {
    let request_type = if typed {
        ": { body: { data: string } }"
    } else {
        ""
    };
    let any = if typed { ": any" } else { "" };
    let _ = write!(
        output,
        "export class Handler {{\n  static __yon_frames__ = true;\n  static async POST(__yon_request__{request_type}) {{\n    const __yon_cache__{any} = [];\n    const __yon_call__{any} = JSON.parse(__yon_request__.body.data);\n    const __yon_target__{any} = __yon_resolve_{target}__(__yon_cache__);\n    switch (__yon_call__.method) {{\n"
    );
    for method in methods {
        let name = quoted(method);
        let _ = writeln!(
            output,
            "      case {name}: return {{ result: (await __yon_target__.{method}(...__yon_call__.arguments)) ?? null }};"
        );
    }
    output.push_str("    }\n    throw new Error('Gateway method is not declared.');\n  }\n}\n");
}

fn python_providers(stripped: &str, plan: &Plan) -> Result<String, String> {
    let mut output = format!("{stripped}\n");
    for (index, provider) in plan.providers.iter().enumerate() {
        python_provider(&mut output, plan, provider, index)?;
    }
    Ok(output)
}

fn python_gateway(output: &mut String, target: usize, methods: &[String]) {
    output.push_str("import inspect as __yon_inspect__\nimport json as __yon_json__\n\n\n");
    output.push_str("def __yon_gateway_result__(value):\n    if __yon_inspect__.isawaitable(value):\n        async def __yon_awaited__():\n            return {\"result\": await value}\n        return __yon_awaited__()\n    return {\"result\": value}\n\n\n");
    let _ = write!(
        output,
        "class Handler:\n    __yon_frames__ = True\n\n    @staticmethod\n    def POST(__yon_request__):\n        __yon_cache__ = {{}}\n        __yon_call__ = __yon_json__.loads(__yon_request__[\"body\"][\"data\"])\n        __yon_target__ = __yon_resolve_{target}__(__yon_cache__)\n"
    );
    for method in methods {
        let name = quoted(method);
        let _ = write!(
            output,
            "        if __yon_call__[\"method\"] == {name}:\n            return __yon_gateway_result__(__yon_target__.{method}(*__yon_call__[\"arguments\"]))\n"
        );
    }
    output.push_str("        raise LookupError(\"Gateway method is not declared.\")\n");
}

fn php_providers(stripped: &str, plan: &Plan) -> Result<String, String> {
    let source = stripped.trim_end().strip_suffix("?>").unwrap_or(stripped);
    let mut output = format!("{source}\n");
    for (index, provider) in plan.providers.iter().enumerate() {
        php_provider(&mut output, plan, provider, index)?;
    }
    Ok(output)
}

fn php_gateway(output: &mut String, target: usize, methods: &[String]) {
    let _ = write!(
        output,
        "class Handler {{\n    const YON_FRAMES = true;\n\n    public static function POST($__yon_request__) {{\n        $__yon_cache__ = [];\n        $__yon_call__ = json_decode($__yon_request__->body(), true);\n        $__yon_target__ = __yon_resolve_{target}__($__yon_cache__);\n        switch ($__yon_call__['method']) {{\n"
    );
    for method in methods {
        let _ = writeln!(
            output,
            "            case '{method}': return ['result' => $__yon_target__->{method}(...$__yon_call__['arguments'])];"
        );
    }
    output.push_str("        }\n        throw new \\RuntimeException('Gateway method is not declared.');\n    }\n}\n");
}

fn quoted(text: &str) -> String {
    serde_json::Value::String(text.to_owned()).to_string()
}

fn provider_index(plan: &Plan, name: &str) -> Result<usize, String> {
    plan.providers
        .iter()
        .position(|provider| provider.name == name)
        .ok_or_else(|| format!("Yon provider {name} is not present in the validated plan."))
}

fn dependencies(plan: &Plan, provider: &Provider, cache: &str) -> Result<String, String> {
    let mut arguments = Vec::with_capacity(provider.dependencies.len());
    for name in &provider.dependencies {
        let index = provider_index(plan, name)?;
        arguments.push(format!("__yon_resolve_{index}__({cache})"));
    }
    Ok(arguments.join(", "))
}

fn javascript(extension: &str, stripped: &str, plan: &Plan) -> Result<String, String> {
    let typed = extension == "ts";
    let controller = provider_index(plan, &plan.controller)?;
    let mut output = format!("{stripped}\n");
    for (index, provider) in plan.providers.iter().enumerate() {
        javascript_provider(&mut output, plan, provider, index, typed)?;
    }
    output.push_str("export class Handler {\n  static __yon_frames__ = true;\n");
    for route in &plan.routes {
        javascript_route(&mut output, plan, route, controller, typed);
    }
    output.push_str("}\n");
    Ok(output)
}

pub(crate) fn socket(
    extension: &str,
    stripped: &str,
    plan: &Plan,
    methods: &SocketMethods,
) -> Result<String, String> {
    match extension {
        "js" | "ts" => javascript_socket(extension, stripped, plan, methods),
        "py" => python_socket(stripped, plan, methods),
        "php" => php_socket(stripped, plan, methods),
        "rb" => RubyLowering::socket(stripped, plan, methods),
        _ => super::compiled::socket(extension, stripped, plan, methods),
    }
}

pub(crate) struct SocketCalls;

impl SocketCalls {
    pub(crate) fn arguments(method: &SocketMethod, values: &[&str]) -> String {
        values[..method.parameters.min(values.len())].join(", ")
    }

    pub(crate) fn message_type(plan: &Plan) -> Option<&str> {
        plan.routes
            .iter()
            .find(|route| route.socket.is_some())
            .and_then(|route| route.parameters.first())
            .map(|parameter| parameter.type_text.as_str())
    }
}

fn python_socket(stripped: &str, plan: &Plan, methods: &SocketMethods) -> Result<String, String> {
    let controller = provider_index(plan, &plan.controller)?;
    let mut output = format!("{stripped}\n");
    for (index, provider) in plan.providers.iter().enumerate() {
        python_provider(&mut output, plan, provider, index)?;
    }
    let message = match SocketCalls::message_type(plan) {
        Some(kind) if super::route_calls::RouteCalls::needs_conversion("py", kind) => {
            format!("YonModels.read({kind}, value)")
        }
        _ => String::from("value"),
    };
    output.push_str("class __YonSocketEndpoint__:\n    def __init__(self, target):\n        self.target = target\n\n");
    for (event, method, values) in [
        ("open", methods.open.as_ref(), ["session", ""]),
        ("close", methods.close.as_ref(), ["session", ""]),
        ("error", methods.error.as_ref(), ["session", "error"]),
    ] {
        let parameters = values
            .iter()
            .filter(|value| !value.is_empty())
            .flat_map(|value| [", ", *value])
            .collect::<String>();
        let body = match method {
            Some(method) => format!(
                "return self.target.{}({})",
                method.name,
                SocketCalls::arguments(method, &values)
            ),
            None => String::from("return None"),
        };
        let _ = write!(
            output,
            "    def {event}(self{parameters}):\n        {body}\n\n"
        );
    }
    let _ = write!(
        output,
        "    def message(self, session, value):\n        return self.target.{}(session, {message})\n\n\nclass Handler:\n    __yon_frames__ = True\n\n    @staticmethod\n    def GET(__yon_request__):\n        __yon_cache__ = {{}}\n        return YonSocket.run(__YonSocketEndpoint__(__yon_resolve_{controller}__(__yon_cache__)), __yon_request__)\n",
        methods.message
    );
    Ok(output)
}

fn php_socket(stripped: &str, plan: &Plan, methods: &SocketMethods) -> Result<String, String> {
    let controller = provider_index(plan, &plan.controller)?;
    let mut output = php_providers(stripped, plan)?;
    let message = match SocketCalls::message_type(plan) {
        Some(kind) if super::route_calls::RouteCalls::needs_conversion("php", kind) => {
            format!("YonModels::read({kind}::class, $value)")
        }
        _ => String::from("$value"),
    };
    let _ = write!(
        output,
        "final class __YonSocketEndpoint implements YonSocketEndpoint {{\n    public function __construct(private {} $target) {{}}\n\n",
        plan.controller
    );
    for (event, method, parameters, values) in [
        (
            "open",
            methods.open.as_ref(),
            "SocketSession $session",
            ["$session", ""],
        ),
        (
            "close",
            methods.close.as_ref(),
            "SocketSession $session",
            ["$session", ""],
        ),
        (
            "error",
            methods.error.as_ref(),
            "SocketSession $session, Throwable $error",
            ["$session", "$error"],
        ),
    ] {
        let body = method.map_or_else(String::new, |method| {
            format!(
                " $this->target->{}({}); ",
                method.name,
                SocketCalls::arguments(method, &values)
            )
        });
        let _ = writeln!(
            output,
            "    public function {event}({parameters}): void {{{body}}}"
        );
    }
    let _ = write!(
        output,
        "    public function message(SocketSession $session, mixed $value): void {{ $this->target->{}($session, {message}); }}\n}}\n\nclass Handler {{\n    const YON_FRAMES = true;\n\n    public static function GET($__yon_request__) {{\n        $__yon_cache__ = [];\n        YonSocket::run(new __YonSocketEndpoint(__yon_resolve_{controller}__($__yon_cache__)), $__yon_request__);\n    }}\n}}\n",
        methods.message
    );
    Ok(output)
}

fn javascript_socket(
    extension: &str,
    stripped: &str,
    plan: &Plan,
    methods: &SocketMethods,
) -> Result<String, String> {
    let typed = extension == "ts";
    let controller = provider_index(plan, &plan.controller)?;
    let mut output = format!("{stripped}\n");
    for (index, provider) in plan.providers.iter().enumerate() {
        javascript_provider(&mut output, plan, provider, index, typed)?;
    }
    let names = serde_json::json!({
        "open": methods.open.as_ref().map(|method| &method.name),
        "message": methods.message,
        "close": methods.close.as_ref().map(|method| &method.name),
        "error": methods.error.as_ref().map(|method| &method.name),
    });
    let (request_type, cache_type, runtime) = if typed {
        (": any", ": unknown[]", "(globalThis as any).YonSocket")
    } else {
        ("", "", "globalThis.YonSocket")
    };
    let _ = write!(
        output,
        "export class Handler {{\n  static __yon_frames__ = true;\n  static GET(__yon_request__{request_type}) {{\n    const __yon_cache__{cache_type} = [];\n    return {runtime}.run(__yon_resolve_{controller}__(__yon_cache__), {names}, __yon_request__);\n  }}\n}}\n"
    );
    Ok(output)
}

fn javascript_route(
    output: &mut String,
    plan: &Plan,
    route: &Route,
    controller: usize,
    typed: bool,
) {
    let modifier = if route.streaming { "async *" } else { "" };
    let argument_type = if !typed {
        String::new()
    } else if route.dispatcher {
        String::from(": any")
    } else if route.takes_request {
        format!(": Parameters<{}['{}']>[0]", plan.controller, route.method)
    } else {
        String::from(": unknown")
    };
    let request = match (route.takes_request, route.dispatcher, typed) {
        (false, _, _) => "",
        (true, true, _) => "__yon_request__",
        (true, false, false) => "YonHttp.request(__yon_request__)",
        (true, false, true) => "(globalThis as any).YonHttp.request(__yon_request__)",
    };
    let errors = super::route_calls::RouteCalls::error_owners(plan)
        .iter()
        .map(|(index, _)| format!("__yon_resolve_{index}__(__yon_cache__)"))
        .collect::<Vec<_>>();
    let invocation = super::route_calls::RouteCalls::invocation_errors(
        route,
        &format!("__yon_resolve_{controller}__(__yon_cache__)"),
        ".",
        request,
        &errors,
    );
    let action = if route.streaming { "yield*" } else { "return" };
    let cache_type = if typed { ": unknown[]" } else { "" };
    let _ = write!(
        output,
        "  static {modifier}{}(__yon_request__{argument_type}) {{\n    const __yon_cache__{cache_type} = [];\n    {action} {invocation};\n  }}\n",
        route.verb
    );
}

fn javascript_provider(
    output: &mut String,
    plan: &Plan,
    provider: &Provider,
    index: usize,
    typed: bool,
) -> Result<(), String> {
    let parameters = if typed {
        "__yon_cache__: unknown[]"
    } else {
        "__yon_cache__"
    };
    let result_type = if typed {
        format!(": {}", provider.name)
    } else {
        String::new()
    };
    let _ = writeln!(
        output,
        "function __yon_resolve_{index}__({parameters}){result_type} {{"
    );
    if provider.scope != Scope::Transient {
        let cast = if typed {
            format!(" as {}", provider.name)
        } else {
            String::new()
        };
        let _ = writeln!(
            output,
            "  if (__yon_cache__[{index}] !== void 0) return __yon_cache__[{index}]{cast};"
        );
    }
    let arguments = dependencies(plan, provider, "__yon_cache__")?;
    let _ = writeln!(
        output,
        "  const __yon_value__ = new {}({arguments});",
        provider.name
    );
    if provider.scope != Scope::Transient {
        let _ = writeln!(output, "  __yon_cache__[{index}] = __yon_value__;");
    }
    output.push_str("  return __yon_value__;\n}\n");
    Ok(())
}

fn python(stripped: &str, plan: &Plan) -> Result<String, String> {
    let controller = provider_index(plan, &plan.controller)?;
    let mut output = format!("{stripped}\n");
    for (index, provider) in plan.providers.iter().enumerate() {
        python_provider(&mut output, plan, provider, index)?;
    }
    output.push_str("class Handler:\n    __yon_frames__ = True\n\n");
    for route in &plan.routes {
        python_route(&mut output, plan, route, controller);
    }
    Ok(output)
}

fn python_provider(
    output: &mut String,
    plan: &Plan,
    provider: &Provider,
    index: usize,
) -> Result<(), String> {
    let _ = writeln!(output, "def __yon_resolve_{index}__(__yon_cache__):");
    if provider.scope != Scope::Transient {
        let _ = write!(
            output,
            "    if {index} in __yon_cache__:\n        return __yon_cache__[{index}]\n"
        );
    }
    let arguments = dependencies(plan, provider, "__yon_cache__")?;
    let _ = writeln!(output, "    __yon_value__ = {}({arguments})", provider.name);
    if provider.scope != Scope::Transient {
        let _ = writeln!(output, "    __yon_cache__[{index}] = __yon_value__");
    }
    output.push_str("    return __yon_value__\n\n");
    Ok(())
}

fn python_route(output: &mut String, plan: &Plan, route: &Route, controller: usize) {
    let request = match (route.takes_request, route.dispatcher) {
        (false, _) => "",
        (true, true) => "__yon_request__",
        (true, false) => "YonHttp.request(__yon_request__)",
    };
    let action = if route.streaming {
        "yield from"
    } else {
        "return"
    };
    let errors = super::route_calls::RouteCalls::error_owners(plan)
        .iter()
        .map(|(index, _)| format!("__yon_resolve_{index}__(__yon_cache__)"))
        .collect::<Vec<_>>();
    let invocation = super::route_calls::RouteCalls::invocation_errors(
        route,
        &format!("__yon_resolve_{controller}__(__yon_cache__)"),
        ".",
        request,
        &errors,
    );
    let _ = write!(
        output,
        "    @staticmethod\n    def {}(__yon_request__):\n        __yon_cache__ = {{}}\n        {action} {invocation}\n\n",
        route.verb
    );
}

fn php(stripped: &str, plan: &Plan) -> Result<String, String> {
    let controller = provider_index(plan, &plan.controller)?;
    let source = stripped.trim_end().strip_suffix("?>").unwrap_or(stripped);
    let mut output = format!("{source}\n");
    for (index, provider) in plan.providers.iter().enumerate() {
        php_provider(&mut output, plan, provider, index)?;
    }
    output.push_str("class Handler {\n    const YON_FRAMES = true;\n\n");
    for route in &plan.routes {
        php_route(&mut output, plan, route, controller);
    }
    output.push_str("}\n");
    Ok(output)
}

fn php_provider(
    output: &mut String,
    plan: &Plan,
    provider: &Provider,
    index: usize,
) -> Result<(), String> {
    let _ = writeln!(
        output,
        "function __yon_resolve_{index}__(array &$__yon_cache__): {} {{",
        provider.name
    );
    if provider.scope != Scope::Transient {
        let _ = writeln!(
            output,
            "    if (array_key_exists({index}, $__yon_cache__)) return $__yon_cache__[{index}];"
        );
    }
    let arguments = dependencies(plan, provider, "$__yon_cache__")?;
    let _ = writeln!(
        output,
        "    $__yon_value__ = new {}({arguments});",
        provider.name
    );
    if provider.scope != Scope::Transient {
        let _ = writeln!(output, "    $__yon_cache__[{index}] = $__yon_value__;");
    }
    output.push_str("    return $__yon_value__;\n}\n");
    Ok(())
}

fn php_route(output: &mut String, plan: &Plan, route: &Route, controller: usize) {
    let request = if route.takes_request {
        "$__yon_request__"
    } else {
        ""
    };
    let action = if route.streaming {
        "yield from"
    } else {
        "return"
    };
    let errors = super::route_calls::RouteCalls::error_owners(plan)
        .iter()
        .map(|(index, _)| format!("__yon_resolve_{index}__($__yon_cache__)"))
        .collect::<Vec<_>>();
    let invocation = super::route_calls::RouteCalls::invocation_errors(
        route,
        &format!("__yon_resolve_{controller}__($__yon_cache__)"),
        "->",
        request,
        &errors,
    );
    let _ = write!(
        output,
        "    public static function {}($__yon_request__) {{\n        $__yon_cache__ = [];\n        {action} {invocation};\n    }}\n",
        route.verb
    );
}

pub(crate) struct RubyLowering;

impl RubyLowering {
    fn program(stripped: &str, plan: &Plan) -> Result<String, String> {
        let controller = provider_index(plan, &plan.controller)?;
        let mut output = Self::providers(stripped, plan)?;
        output.push_str("class Handler\n  YON_FRAMES = true\n\n");
        for route in &plan.routes {
            Self::route(&mut output, plan, route, controller);
        }
        output.push_str("end\n");
        Ok(output)
    }

    fn providers(stripped: &str, plan: &Plan) -> Result<String, String> {
        Ok(format!("{stripped}\n{}", Self::provider_module(plan)?))
    }

    fn socket(stripped: &str, plan: &Plan, methods: &SocketMethods) -> Result<String, String> {
        let controller = provider_index(plan, &plan.controller)?;
        let mut output = Self::providers(stripped, plan)?;
        let message = SocketCalls::message_type(plan)
            .and_then(|kind| crate::route_types::TypeText::shape("rb", kind).ok())
            .and_then(|shape| super::route_calls::RouteCalls::ruby_descriptor(&shape))
            .map_or_else(
                || String::from("value"),
                |descriptor| {
                    format!("YonModels.read({descriptor}, value, YonGenerated.field_types)")
                },
            );
        output.push_str("module YonGenerated\n  class SocketEndpoint\n    def initialize(target)\n      @target = target\n    end\n");
        for (event, method, values) in [
            ("open", methods.open.as_ref(), ["session", ""]),
            ("close", methods.close.as_ref(), ["session", ""]),
            ("error", methods.error.as_ref(), ["session", "error"]),
        ] {
            let parameters = values
                .iter()
                .filter(|value| !value.is_empty())
                .copied()
                .collect::<Vec<_>>()
                .join(", ");
            let body = method.map_or_else(
                || String::from("nil"),
                |method| {
                    format!(
                        "@target.{}({})",
                        method.name,
                        SocketCalls::arguments(method, &values)
                    )
                },
            );
            let _ = write!(
                output,
                "\n    def {event}({parameters})\n      {body}\n    end\n"
            );
        }
        let _ = write!(
            output,
            "\n    def message(session, value)\n      @target.{}(session, {message})\n    end\n  end\nend\n\nclass Handler\n  YON_FRAMES = true\n\n  def self.GET(request)\n    cache = {{}}\n    YonSocket.run(YonGenerated::SocketEndpoint.new(YonGenerated.resolve_{controller}(cache)), request)\n  end\nend\n",
            methods.message
        );
        Ok(output)
    }

    pub(crate) fn provider_module(plan: &Plan) -> Result<String, String> {
        let mut output = String::from("module YonGenerated\n");
        for (index, provider) in plan.providers.iter().enumerate() {
            Self::provider(&mut output, plan, provider, index)?;
        }
        output.push_str("end\n");
        Ok(output)
    }

    fn provider(
        output: &mut String,
        plan: &Plan,
        provider: &Provider,
        index: usize,
    ) -> Result<(), String> {
        let cached = provider.scope != Scope::Transient;
        let _ = writeln!(output, "  def self.resolve_{index}(cache)");
        if cached {
            let _ = writeln!(output, "    return cache[{index}] if cache.key?({index})");
        }
        let mut arguments = Vec::with_capacity(provider.dependencies.len());
        for name in &provider.dependencies {
            let dependency = provider_index(plan, name)?;
            arguments.push(format!("resolve_{dependency}(cache)"));
        }
        if provider.injections.is_empty() {
            let _ = writeln!(
                output,
                "    value = {}.new({})",
                provider.name,
                arguments.join(", ")
            );
        } else {
            let _ = writeln!(output, "    value = {}.new", provider.name);
            for (injection, argument) in provider.injections.iter().zip(&arguments) {
                let _ = writeln!(
                    output,
                    "    value.instance_variable_set(:@{}, {argument})",
                    injection.field
                );
            }
        }
        if cached {
            let _ = writeln!(output, "    cache[{index}] = value");
        }
        output.push_str("    value\n  end\n\n");
        Ok(())
    }

    fn route(output: &mut String, plan: &Plan, route: &Route, controller: usize) {
        let request = match (route.takes_request, route.dispatcher) {
            (false, _) => "",
            (true, true) => "request",
            (true, false) => "YonHttp.request(request)",
        };
        let errors = super::route_calls::RouteCalls::error_owners(plan)
            .iter()
            .map(|(index, _)| format!("YonGenerated.resolve_{index}(cache)"))
            .collect::<Vec<_>>();
        let invocation = super::route_calls::RouteCalls::invocation_errors(
            route,
            &format!("YonGenerated.resolve_{controller}(cache)"),
            ".",
            request,
            &errors,
        );
        let _ = writeln!(output, "  def self.{}(request)", route.verb);
        if route.streaming {
            let _ = writeln!(
                output,
                "    Enumerator.new do |events|\n      cache = {{}}\n      {invocation} {{ |event| events << event }}\n    end"
            );
        } else {
            let _ = writeln!(output, "    cache = {{}}\n    {invocation}");
        }
        output.push_str("  end\n\n");
    }

    fn gateway(output: &mut String, target: usize, methods: &[String]) {
        let _ = write!(
            output,
            "class Handler\n  YON_FRAMES = true\n\n  def self.POST(request)\n    cache = {{}}\n    call = JSON.parse(request[\"body\"][\"data\"])\n    target = YonGenerated.resolve_{target}(cache)\n"
        );
        for method in methods {
            let name = quoted(method);
            let _ = writeln!(
                output,
                "    return {{ \"result\" => target.{method}(*call[\"arguments\"]) }} if call[\"method\"] == {name}"
            );
        }
        output.push_str("    raise YonNoRoute, \"Gateway method is not declared.\"\n  end\nend\n");
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::yon::plan::{Role, Route};

    fn plan() -> Plan {
        Plan {
            errors: Vec::new(),
            providers: vec![Provider {
                name: "Products".to_owned(),
                role: Role::Controller,
                scope: Scope::Request,
                dependencies: Vec::new(),
                injections: Vec::new(),
            }],
            controller: "Products".to_owned(),
            routes: vec![Route {
                verb: "GET".to_owned(),
                method: "list".to_owned(),
                streaming: false,
                takes_request: true,
                ..Route::default()
            }],
        }
    }

    #[test]
    fn lowering_is_deterministic_and_keeps_authored_source() {
        for (extension, source) in [
            ("js", "class Products { list(request) { return request; } }"),
            (
                "ts",
                "class Products { list(request: unknown) { return request; } }",
            ),
            (
                "py",
                "class Products:\n    def list(self, request): return request\n",
            ),
            (
                "php",
                "<?php\nclass Products { function listing($request) { return $request; } }\n",
            ),
            (
                "rb",
                "class Products\n  def list(request)\n    request\n  end\nend\n",
            ),
        ] {
            let first = lower(extension, source, &plan()).expect("lowered source");
            assert!(first.starts_with(source), "{extension}");
            assert_eq!(first, lower(extension, source, &plan()).expect("repeat"));
        }
    }

    #[test]
    fn invalid_internal_plans_fail_without_panicking() {
        assert!(lower("unknown", "", &plan()).is_err());
        for extension in ["js", "ts", "py", "php", "rb"] {
            let mut missing_controller = plan();
            missing_controller.controller = "Missing".to_owned();
            assert!(lower(extension, "", &missing_controller).is_err());
            let mut missing_dependency = plan();
            missing_dependency.providers[0]
                .dependencies
                .push("Missing".to_owned());
            assert!(lower(extension, "", &missing_dependency).is_err());
        }
    }
}
