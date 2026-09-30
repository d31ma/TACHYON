use super::{EventModule, Plan, Write, index, quote};
use crate::events::{EventListener, EventMethod, EventType};
use crate::yon::plan::Route;
use crate::yon::route_calls::RouteCalls;

pub(super) fn lower(extension: &str, plan: &Plan, events: &EventModule) -> Result<String, String> {
    match extension {
        "js" | "ts" => javascript(plan, events),
        "py" => python(plan, events),
        "php" => php(plan, events),
        "java" | "cs" => managed(extension, plan, events),
        "kt" => kotlin(plan, events),
        "rs" => rust(plan, events),
        "dart" => dart(plan, events),
        "go" => go(plan, events),
        "rb" => ruby(plan, events),
        _ => Err("Unknown event container language".into()),
    }
}

fn eager(plan: &Plan, events: &EventModule) -> Result<Vec<usize>, String> {
    let mut indexes = vec![index(plan, &plan.controller)?];
    for (index, _) in RouteCalls::error_owners(plan) {
        if !indexes.contains(&index) {
            indexes.push(index);
        }
    }
    for event_type in &events.types {
        let index = index(plan, &event_type.name)?;
        if !indexes.contains(&index) {
            indexes.push(index);
        }
    }
    Ok(indexes)
}

fn dependency_calls(
    plan: &Plan,
    dependencies: &[String],
    prefix: &str,
    suffix: &str,
) -> Result<String, String> {
    let mut calls = Vec::new();
    for name in dependencies {
        let index = index(plan, name)?;
        calls.push(format!("{prefix}{index}{suffix}"));
    }
    Ok(calls.join(","))
}

fn listener_method<'a>(
    events: &'a EventModule,
    event_type: &EventType,
    listener: &EventListener,
) -> Result<&'a EventMethod, String> {
    let found = events
        .methods
        .iter()
        .find(|method| method.owner == event_type.name && method.name == listener.method);
    let Some(method) = found else {
        return Err("Missing listener signature".into());
    };
    Ok(method)
}

fn request_argument(route: &Route, name: &'static str) -> &'static str {
    if route.takes_request { name } else { "" }
}

fn javascript(plan: &Plan, events: &EventModule) -> Result<String, String> {
    let mut out = String::new();
    for (i, provider) in plan.providers.iter().enumerate() {
        let args = dependency_calls(plan, &provider.dependencies, "__eventGet", "(cache,order)")?;
        let _ = writeln!(
            out,
            "function __eventGet{i}(cache,order) {{ if (cache[{i}]) return cache[{i}]; const value = new {}({args}); cache[{i}] = value; order.push({i}); return value; }}",
            provider.name
        );
    }
    let _ = writeln!(out, "__YonEvents.topics = new Set({});", topic_list(events));
    out.push_str("export class Handler {\nstatic __yon_frames__ = true;\n");
    for route in &plan.routes {
        javascript_route(&mut out, plan, events, route)?;
    }
    if !events.topic_listeners().is_empty() {
        out.push_str("static async __yonDeliver(delivery) { const events = new __YonEvents(new Set(delivery.listeners)); const cache = [], order = []; try {\n");
        javascript_setup(&mut out, plan, events)?;
        out.push_str("events.queue.push([delivery.topic, JSON.stringify(delivery.payload)]); await events.drain(); return null; } finally { await events.close(); } }\n");
    }
    out.push_str("}\n");
    Ok(out)
}

fn listener_key(event_type: &EventType, listener: &EventListener) -> String {
    quote(&format!("{}.{}", event_type.name, listener.method))
}

fn topic_list(events: &EventModule) -> String {
    serde_json::to_string(&events.topics).unwrap_or_else(|_| String::from("[]"))
}

fn javascript_setup(out: &mut String, plan: &Plan, events: &EventModule) -> Result<(), String> {
    for i in eager(plan, events)? {
        let _ = writeln!(out, "__eventGet{i}(cache,order);");
    }
    for event_type in &events.types {
        let owner = index(plan, &event_type.name)?;
        for listener in &event_type.listeners {
            let _ = writeln!(
                out,
                "events.listen({}, value => cache[{owner}].{}(value), {}, {});",
                listener_options(listener),
                listener.method,
                listener.replay,
                quote(&format!("{}.{}", event_type.name, listener.method))
            );
        }
    }
    out.push_str("for (const index of order) { switch(index) {\n");
    for event_type in &events.types {
        let owner = index(plan, &event_type.name)?;
        let _ = writeln!(out, "case {owner}:");
        if let Some(method) = &event_type.post_construct {
            let _ = writeln!(out, "await cache[{owner}].{method}();");
        }
        if let Some(method) = &event_type.pre_destroy {
            let _ = writeln!(
                out,
                "events.destroyers.push(() => cache[{owner}].{method}());"
            );
        }
        out.push_str("break;\n");
    }
    out.push_str("} }\n");
    Ok(())
}

fn javascript_route(
    out: &mut String,
    plan: &Plan,
    events: &EventModule,
    route: &Route,
) -> Result<(), String> {
    let controller = index(plan, &plan.controller)?;
    let _ = writeln!(
        out,
        "static async {}(request) {{ const events = new __YonEvents(); const cache = [], order = []; try {{",
        route.verb
    );
    javascript_setup(out, plan, events)?;
    let request = request_argument(route, "request");
    let errors = RouteCalls::error_owners(plan)
        .iter()
        .map(|(index, _)| format!("cache[{index}]"))
        .collect::<Vec<_>>();
    let invocation = RouteCalls::invocation_errors(
        route,
        &format!("cache[{controller}]"),
        ".",
        request,
        &errors,
    );
    let _ = writeln!(
        out,
        "await events.drain(); const result = await {invocation}; await events.drain(); return result; }} finally {{ await events.close(); }} }}"
    );
    Ok(())
}

fn python(plan: &Plan, events: &EventModule) -> Result<String, String> {
    let mut out = String::new();
    for (i, provider) in plan.providers.iter().enumerate() {
        let args = dependency_calls(plan, &provider.dependencies, "_event_get_", "(cache,order)")?;
        let _ = write!(
            out,
            "def _event_get_{i}(cache,order):\n    if {i} in cache:\n        return cache[{i}]\n    value = {}({args})\n    cache[{i}] = value\n    order.append({i})\n    return value\n",
            provider.name
        );
    }
    let _ = writeln!(out, "_YonEvents.topics = frozenset({})", topic_list(events));
    out.push_str("class Handler:\n    __yon_frames__ = True\n");
    for route in &plan.routes {
        python_route(&mut out, plan, events, route)?;
    }
    if !events.topic_listeners().is_empty() {
        out.push_str("    @staticmethod\n    async def _yon_deliver(delivery):\n        events = _YonEvents(frozenset(delivery['listeners']))\n        cache, order = {}, []\n        try:\n");
        python_setup(&mut out, plan, events)?;
        out.push_str("            events.queue.append((delivery['topic'], _event_json.dumps(delivery['payload'])))\n            await events.drain()\n            return None\n        finally:\n            await events.close()\n");
    }
    Ok(out)
}

fn python_setup(out: &mut String, plan: &Plan, events: &EventModule) -> Result<(), String> {
    for i in eager(plan, events)? {
        let _ = writeln!(out, "            _event_get_{i}(cache,order)");
    }
    for event_type in &events.types {
        let owner = index(plan, &event_type.name)?;
        for listener in &event_type.listeners {
            let replay = if listener.replay { "True" } else { "False" };
            let method = listener_method(events, event_type, listener)?;
            let _ = writeln!(
                out,
                "            events.listen(_event_json.loads({}), lambda value: cache[{owner}].{}(YonModels.read({}, value)), {replay}, {})",
                quote(&listener_options(listener)),
                listener.method,
                payload_type(method),
                quote(&format!("{}.{}", event_type.name, listener.method))
            );
        }
    }
    out.push_str("            for index in order:\n                pass\n");
    for event_type in &events.types {
        let owner = index(plan, &event_type.name)?;
        let _ = writeln!(out, "                if index == {owner}:");
        out.push_str("                    pass\n");
        if let Some(method) = &event_type.post_construct {
            let _ = write!(
                out,
                "                    initialized = cache[{owner}].{method}()\n                    if _event_inspect.isawaitable(initialized):\n                        await initialized\n"
            );
        }
        if let Some(method) = &event_type.pre_destroy {
            let _ = writeln!(
                out,
                "                    events.destroyers.append(cache[{owner}].{method})"
            );
        }
    }
    Ok(())
}

fn python_route(
    out: &mut String,
    plan: &Plan,
    events: &EventModule,
    route: &Route,
) -> Result<(), String> {
    let controller = index(plan, &plan.controller)?;
    let _ = write!(
        out,
        "    @staticmethod\n    async def {}(request):\n        events = _YonEvents()\n        cache, order = {{}}, []\n        try:\n",
        route.verb
    );
    python_setup(out, plan, events)?;
    let request = request_argument(route, "request");
    let errors = RouteCalls::error_owners(plan)
        .iter()
        .map(|(index, _)| format!("cache[{index}]"))
        .collect::<Vec<_>>();
    let invocation = RouteCalls::invocation_errors(
        route,
        &format!("cache[{controller}]"),
        ".",
        request,
        &errors,
    );
    let _ = write!(
        out,
        "            await events.drain()\n            result = {invocation}\n            if _event_inspect.isawaitable(result):\n                result = await result\n            await events.drain()\n            return result\n        finally:\n            await events.close()\n"
    );
    Ok(())
}

fn php(plan: &Plan, events: &EventModule) -> Result<String, String> {
    let mut out =
        String::from("class __EventContainer { public $cache = []; public $order = [];\n");
    for (i, provider) in plan.providers.iter().enumerate() {
        let args = dependency_calls(plan, &provider.dependencies, "$this->get", "()")?;
        let _ = writeln!(
            out,
            "function get{i}() {{ if (array_key_exists({i},$this->cache)) return $this->cache[{i}]; $value = new {}({args}); $this->cache[{i}] = $value; $this->order[] = {i}; return $value; }}",
            provider.name
        );
    }
    out.push_str("}\nclass Handler {\nconst YON_FRAMES = true;\n");
    for route in &plan.routes {
        php_route(&mut out, plan, events, route)?;
    }
    if !events.topic_listeners().is_empty() {
        let _ = writeln!(
            out,
            "static function yonDeliver($request) {{ $events = __YonEvents::delivery($request->body(), [{}]); $c = new __EventContainer(); try {{",
            topic_arguments(events)
        );
        php_setup(&mut out, plan, events)?;
        out.push_str("$events->receiveDelivery(); $events->drain(); return null; } finally { $events->close(); } }\n");
    }
    out.push_str("}\n");
    Ok(out)
}

fn php_setup(out: &mut String, plan: &Plan, events: &EventModule) -> Result<(), String> {
    for i in eager(plan, events)? {
        let _ = writeln!(out, "$c->get{i}();");
    }
    for event_type in &events.types {
        let owner = index(plan, &event_type.name)?;
        for listener in &event_type.listeners {
            let method = listener_method(events, event_type, listener)?;
            let _ = writeln!(
                out,
                "$events->listen({}, fn($value) => $c->cache[{owner}]->{}(YonModels::read({}::class, $value)), {}, {});",
                php_literal(&listener_options(listener)),
                listener.method,
                payload_type(method),
                listener.replay,
                listener_key(event_type, listener)
            );
        }
    }
    out.push_str("foreach ($c->order as $index) { switch ($index) {\n");
    for event_type in &events.types {
        let owner = index(plan, &event_type.name)?;
        let _ = writeln!(out, "case {owner}:");
        if let Some(method) = &event_type.post_construct {
            let _ = writeln!(out, "$c->cache[{owner}]->{method}();");
        }
        if let Some(method) = &event_type.pre_destroy {
            let _ = writeln!(
                out,
                "$events->destroyers[] = [$c->cache[{owner}], {}];",
                quote(method)
            );
        }
        out.push_str("break;\n");
    }
    out.push_str("} }\n");
    Ok(())
}

fn ruby(plan: &Plan, events: &EventModule) -> Result<String, String> {
    let mut out = crate::yon::dynamic::RubyLowering::provider_module(plan)?;
    let _ = writeln!(out, "YonEvents.topics = {}", topic_list(events));
    out.push_str("class Handler\n  YON_FRAMES = true\n");
    for route in &plan.routes {
        ruby_route(&mut out, plan, events, route)?;
    }
    if !events.topic_listeners().is_empty() {
        out.push_str("  def self.yon_deliver(delivery)\n    bus = YonEvents.new(delivery[\"listeners\"])\n    cache = {}\n    begin\n");
        ruby_setup(&mut out, plan, events)?;
        out.push_str("      bus.receive(delivery[\"topic\"], JSON.generate(delivery[\"payload\"]))\n      bus.drain\n      nil\n    ensure\n      bus.close\n    end\n  end\n");
    }
    out.push_str("end\n");
    Ok(out)
}

fn ruby_setup(out: &mut String, plan: &Plan, events: &EventModule) -> Result<(), String> {
    for i in eager(plan, events)? {
        let _ = writeln!(out, "      YonGenerated.resolve_{i}(cache)");
    }
    for event_type in &events.types {
        let owner = index(plan, &event_type.name)?;
        for listener in &event_type.listeners {
            let method = listener_method(events, event_type, listener)?;
            let _ = writeln!(
                out,
                "      bus.listen({}, lambda {{ |value| cache[{owner}].{}(YonModels.read({}, value, YonGenerated.respond_to?(:field_types) ? YonGenerated.field_types : {{}})) }}, {}, {})",
                php_literal(&listener_options(listener)),
                listener.method,
                payload_type(method),
                listener.replay,
                quote(&format!("{}.{}", event_type.name, listener.method))
            );
        }
    }
    if events.types.is_empty() {
        return Ok(());
    }
    out.push_str("      cache.keys.each do |index|\n        case index\n");
    for event_type in &events.types {
        let owner = index(plan, &event_type.name)?;
        let _ = writeln!(out, "        when {owner}");
        if let Some(method) = &event_type.post_construct {
            let _ = writeln!(out, "          cache[{owner}].{method}");
        }
        if let Some(method) = &event_type.pre_destroy {
            let _ = writeln!(
                out,
                "          bus.destroyers << lambda {{ cache[{owner}].{method} }}"
            );
        }
    }
    out.push_str("        end\n      end\n");
    Ok(())
}

fn ruby_route(
    out: &mut String,
    plan: &Plan,
    events: &EventModule,
    route: &Route,
) -> Result<(), String> {
    let controller = index(plan, &plan.controller)?;
    let _ = writeln!(
        out,
        "  def self.{}(request)\n    bus = YonEvents.new\n    cache = {{}}\n    begin",
        route.verb
    );
    ruby_setup(out, plan, events)?;
    let request = request_argument(route, "request");
    let errors = RouteCalls::error_owners(plan)
        .iter()
        .map(|(index, _)| format!("YonGenerated.resolve_{index}(cache)"))
        .collect::<Vec<_>>();
    let invocation = RouteCalls::invocation_errors(
        route,
        &format!("YonGenerated.resolve_{controller}(cache)"),
        ".",
        request,
        &errors,
    );
    let _ = writeln!(
        out,
        "      bus.drain\n      result = {invocation}\n      bus.drain\n      result\n    ensure\n      bus.close\n    end\n  end"
    );
    Ok(())
}

fn php_route(
    out: &mut String,
    plan: &Plan,
    events: &EventModule,
    route: &Route,
) -> Result<(), String> {
    let controller = index(plan, &plan.controller)?;
    let _ = writeln!(
        out,
        "static function {}($request) {{ $events = new __YonEvents([{}]); $c = new __EventContainer(); try {{",
        route.verb,
        topic_arguments(events)
    );
    php_setup(out, plan, events)?;
    let request = request_argument(route, "$request");
    let errors = RouteCalls::error_owners(plan)
        .iter()
        .map(|(index, _)| format!("$c->cache[{index}]"))
        .collect::<Vec<_>>();
    let invocation = RouteCalls::invocation_errors(
        route,
        &format!("$c->cache[{controller}]"),
        "->",
        request,
        &errors,
    );
    let _ = writeln!(
        out,
        "$events->drain(); $result = {invocation}; $events->drain(); return $result; }} finally {{ $events->close(); }} }}"
    );
    Ok(())
}

struct ManagedSyntax {
    throws: &'static str,
    order_list: &'static str,
    access: &'static str,
    add: &'static str,
    object: &'static str,
    listen: &'static str,
    arrow: &'static str,
    for_loop: &'static str,
    drain: &'static str,
    close: &'static str,
    deliver: &'static str,
    delivery: &'static str,
    body: &'static str,
    receive: &'static str,
    is_java: bool,
}

impl ManagedSyntax {
    const JAVA: Self = Self {
        throws: " throws Exception",
        order_list: "final java.util.List<Integer> order = new java.util.ArrayList<>();",
        access: "",
        add: "add",
        object: "Object",
        listen: "listen",
        arrow: "->",
        for_loop: "for (int index : c.order)",
        drain: "drain",
        close: "close",
        deliver: "__yonDeliver",
        delivery: "delivery",
        body: "body",
        receive: "receiveDelivery",
        is_java: true,
    };
    const CSHARP: Self = Self {
        throws: "",
        order_list: "internal readonly System.Collections.Generic.List<int> order = new();",
        access: "internal ",
        add: "Add",
        object: "object",
        listen: "Listen",
        arrow: "=>",
        for_loop: "foreach (int index in c.order)",
        drain: "Drain",
        close: "Dispose",
        deliver: "__YonDeliver",
        delivery: "Delivery",
        body: "Body",
        receive: "ReceiveDelivery",
        is_java: false,
    };
}

fn managed(extension: &str, plan: &Plan, events: &EventModule) -> Result<String, String> {
    let syntax = if extension == "java" {
        &ManagedSyntax::JAVA
    } else {
        &ManagedSyntax::CSHARP
    };
    let access = syntax.access;
    let throws = syntax.throws;
    let mut out = String::from("class __EventContainer {\n");
    out.push_str(syntax.order_list);
    for (i, provider) in plan.providers.iter().enumerate() {
        let args = dependency_calls(plan, &provider.dependencies, "get", "()")?;
        let _ = writeln!(
            out,
            "{access}{} p{i}; {access}{} get{i}(){throws} {{ if (p{i} != null) return p{i}; p{i} = new {}({args}); order.{}({i}); return p{i}; }}",
            provider.name, provider.name, provider.name, syntax.add
        );
    }
    out.push_str("}\nclass Handler {\n");
    for route in &plan.routes {
        managed_route(&mut out, syntax, plan, events, route)?;
    }
    if !events.topic_listeners().is_empty() {
        managed_delivery(&mut out, syntax, plan, events)?;
    }
    out.push_str("}\n");
    Ok(out)
}

fn managed_delivery(
    out: &mut String,
    syntax: &ManagedSyntax,
    plan: &Plan,
    events: &EventModule,
) -> Result<(), String> {
    let _ = writeln!(
        out,
        "public static {} {}(__YonFrame request){} {{ var events = __YonEvents.{}(request.{}(), {}); var c = new __EventContainer(); try {{",
        syntax.object,
        syntax.deliver,
        syntax.throws,
        syntax.delivery,
        syntax.body,
        topic_arguments(events)
    );
    managed_setup(out, syntax, plan, events)?;
    let _ = writeln!(
        out,
        "events.{}(); events.{}(); return null; }} finally {{ events.{}(); }} }}",
        syntax.receive, syntax.drain, syntax.close
    );
    Ok(())
}

fn topic_arguments(events: &EventModule) -> String {
    let names: Vec<String> = events.topics.iter().map(|name| quote(name)).collect();
    names.join(", ")
}

fn managed_setup(
    out: &mut String,
    syntax: &ManagedSyntax,
    plan: &Plan,
    events: &EventModule,
) -> Result<(), String> {
    for i in eager(plan, events)? {
        let _ = writeln!(out, "c.get{i}();");
    }
    managed_listeners(out, syntax, plan, events)?;
    let _ = writeln!(out, "{} {{ switch(index) {{", syntax.for_loop);
    managed_lifecycle(out, syntax, plan, events)?;
    out.push_str("} }\n");
    Ok(())
}

fn managed_route(
    out: &mut String,
    syntax: &ManagedSyntax,
    plan: &Plan,
    events: &EventModule,
    route: &Route,
) -> Result<(), String> {
    let controller = index(plan, &plan.controller)?;
    let _ = writeln!(
        out,
        "public static {} {}(__YonFrame request){} {{ var events = new __YonEvents({}); var c = new __EventContainer(); try {{",
        syntax.object,
        route.verb,
        syntax.throws,
        topic_arguments(events)
    );
    managed_setup(out, syntax, plan, events)?;
    let request = request_argument(route, "request");
    let drain = syntax.drain;
    let errors = RouteCalls::error_owners(plan)
        .iter()
        .map(|(index, _)| format!("c.p{index}"))
        .collect::<Vec<_>>();
    let invocation =
        RouteCalls::invocation_errors(route, &format!("c.p{controller}"), ".", request, &errors);
    let _ = writeln!(
        out,
        "events.{drain}(); var result = {invocation}; events.{drain}(); return result; }} finally {{ events.{}(); }} }}",
        syntax.close
    );
    Ok(())
}

fn managed_listeners(
    out: &mut String,
    syntax: &ManagedSyntax,
    plan: &Plan,
    events: &EventModule,
) -> Result<(), String> {
    for event_type in &events.types {
        let owner = index(plan, &event_type.name)?;
        for listener in &event_type.listeners {
            let method = listener_method(events, event_type, listener)?;
            let payload = payload_type(method);
            let reader = if syntax.is_java {
                format!("({payload}) YonModels.read({payload}.class, value)")
            } else {
                format!(
                    "System.Text.Json.JsonSerializer.Deserialize<{payload}>(value, YonModels.Options)"
                )
            };
            let call = format!("c.p{owner}.{}({reader})", listener.method);
            let body = if method.return_type.trim() == "void" {
                format!("{{ {call}; return null; }}")
            } else {
                call
            };
            let _ = writeln!(
                out,
                "events.{}({}, value {} {body}, {}, {});",
                syntax.listen,
                literal(&listener_options(listener), false),
                syntax.arrow,
                listener.replay,
                listener_key(event_type, listener)
            );
        }
    }
    Ok(())
}

fn managed_lifecycle(
    out: &mut String,
    syntax: &ManagedSyntax,
    plan: &Plan,
    events: &EventModule,
) -> Result<(), String> {
    for event_type in &events.types {
        let owner = index(plan, &event_type.name)?;
        let _ = writeln!(out, "case {owner}:");
        if let Some(method) = &event_type.post_construct {
            let _ = writeln!(out, "c.p{owner}.{method}();");
        }
        if let Some(method) = &event_type.pre_destroy {
            let closure = if syntax.is_java {
                format!("c.p{owner}::{method}")
            } else {
                format!("() => {{ c.p{owner}.{method}(); }}")
            };
            let _ = writeln!(out, "events.destroyers.{}({closure});", syntax.add);
        }
        out.push_str("break;\n");
    }
    Ok(())
}

fn kotlin(plan: &Plan, events: &EventModule) -> Result<String, String> {
    let mut out =
        String::from("private class __EventContainer {\nval order = mutableListOf<Int>()\n");
    for (i, provider) in plan.providers.iter().enumerate() {
        let args = dependency_calls(plan, &provider.dependencies, "get", "()")?;
        let _ = writeln!(
            out,
            "val p{i}: {} by lazy {{ val value = {}({args}); order.add({i}); value }}\nfun get{i}(): {} = p{i}",
            provider.name, provider.name, provider.name
        );
    }
    out.push_str("}\nobject Handler {\n");
    for route in &plan.routes {
        kotlin_route(&mut out, plan, events, route)?;
    }
    if !events.topic_listeners().is_empty() {
        let _ = writeln!(
            out,
            "fun __yonDeliver(request: __YonFrame): Any? {{ val events = __YonEvents.delivery(request.body(), {}); val c = __EventContainer(); try {{",
            topic_arguments(events)
        );
        kotlin_setup(&mut out, plan, events)?;
        out.push_str(
            "events.receiveDelivery()\nevents.drain()\nreturn null\n} finally { events.close() } }\n",
        );
    }
    out.push_str("}\n");
    Ok(out)
}

fn kotlin_route(
    out: &mut String,
    plan: &Plan,
    events: &EventModule,
    route: &Route,
) -> Result<(), String> {
    let controller = index(plan, &plan.controller)?;
    let _ = writeln!(
        out,
        "fun {}(request: __YonFrame): Any? {{ val events = __YonEvents({}); val c = __EventContainer(); try {{",
        route.verb,
        topic_arguments(events)
    );
    kotlin_setup(out, plan, events)?;
    let request = request_argument(route, "request");
    let errors = RouteCalls::error_owners(plan)
        .iter()
        .map(|(index, _)| format!("c.p{index}"))
        .collect::<Vec<_>>();
    let invocation =
        RouteCalls::invocation_errors(route, &format!("c.p{controller}"), ".", request, &errors);
    let _ = writeln!(
        out,
        "events.drain()\nval result = {invocation}\nevents.drain()\nreturn result\n}} finally {{ events.close() }} }}"
    );
    Ok(())
}

fn kotlin_setup(out: &mut String, plan: &Plan, events: &EventModule) -> Result<(), String> {
    for i in eager(plan, events)? {
        let _ = writeln!(out, "c.get{i}()");
    }
    for event_type in &events.types {
        let owner = index(plan, &event_type.name)?;
        for listener in &event_type.listeners {
            let method = listener_method(events, event_type, listener)?;
            let options = literal(&listener_options(listener), true);
            let name = &listener.method;
            let payload = payload_type(method);
            let replay = listener.replay;
            let key = listener_key(event_type, listener);
            let _ = writeln!(
                out,
                "events.listen({options}, {{ value -> c.p{owner}.{name}(YonModels.read({payload}::class.java, value) as {payload}) }}, {replay}, {key})"
            );
        }
    }
    out.push_str("for (index in c.order) { when(index) {\n");
    for event_type in &events.types {
        let owner = index(plan, &event_type.name)?;
        let _ = writeln!(out, "{owner} -> {{");
        if let Some(method) = &event_type.post_construct {
            let _ = writeln!(out, "c.p{owner}.{method}()");
        }
        if let Some(method) = &event_type.pre_destroy {
            let _ = writeln!(out, "events.destroyers.add {{ c.p{owner}.{method}() }}");
        }
        out.push_str("}\n");
    }
    out.push_str("} }\n");
    Ok(())
}

fn dart(plan: &Plan, events: &EventModule) -> Result<String, String> {
    let mut out = String::from("final class __EventContainer {\n  final List<int> order = [];\n");
    for (i, provider) in plan.providers.iter().enumerate() {
        let args = dependency_calls(plan, &provider.dependencies, "get", "()")?;
        let _ = writeln!(
            out,
            "  {}? _p{i};\n  {} get p{i} => get{i}();\n  {} get{i}() {{\n    final existing = _p{i};\n    if (existing != null) return existing;\n    final value = {}({args});\n    order.add({i});\n    return _p{i} = value;\n  }}",
            provider.name, provider.name, provider.name, provider.name
        );
    }
    out.push_str("}\n\nfinal class Handler {\n");
    let controller = index(plan, &plan.controller)?;
    for route in &plan.routes {
        dart_route(&mut out, plan, events, route, controller)?;
    }
    if !events.topic_listeners().is_empty() {
        let _ = writeln!(
            out,
            "  Future<Object?> __yonDeliver(__YonFrame request) async {{\n    final events = __YonEvents.delivery(request.body(), [{}]);\n    final c = __EventContainer();\n    try {{",
            dart_topics(events)
        );
        dart_setup(&mut out, plan, events)?;
        out.push_str("      events.receiveDelivery();\n      await events.drain();\n      return null;\n    } finally {\n      events.close();\n    }\n  }\n");
    }
    out.push_str("}\n");
    Ok(out)
}

fn dart_topics(events: &EventModule) -> String {
    let names: Vec<String> = events
        .topics
        .iter()
        .map(|name| dart_literal(name))
        .collect();
    names.join(", ")
}

fn dart_setup(out: &mut String, plan: &Plan, events: &EventModule) -> Result<(), String> {
    for i in eager(plan, events)? {
        let _ = writeln!(out, "      c.get{i}();");
    }
    dart_listeners(out, plan, events)?;
    out.push_str("      for (final index in c.order) {\n        switch (index) {\n");
    dart_lifecycle(out, plan, events)?;
    out.push_str("        }\n      }\n");
    Ok(())
}

fn dart_route(
    out: &mut String,
    plan: &Plan,
    events: &EventModule,
    route: &Route,
    controller: usize,
) -> Result<(), String> {
    let _ = writeln!(
        out,
        "  Future<Object?> {}(__YonFrame request) async {{\n    final events = __YonEvents([{}]);\n    final c = __EventContainer();\n    try {{",
        route.verb,
        dart_topics(events)
    );
    dart_setup(out, plan, events)?;
    let request = request_argument(route, "request");
    let errors = RouteCalls::error_owners(plan)
        .iter()
        .map(|(index, _)| format!("c.p{index}"))
        .collect::<Vec<_>>();
    let invocation =
        RouteCalls::invocation_errors(route, &format!("c.p{controller}"), ".", request, &errors);
    let _ = writeln!(
        out,
        "      await events.drain();\n      Object? result = {invocation};\n      if (result is Future) result = await result;\n      await events.drain();\n      return result;\n    }} finally {{\n      events.close();\n    }}\n  }}"
    );
    Ok(())
}

fn dart_listeners(out: &mut String, plan: &Plan, events: &EventModule) -> Result<(), String> {
    for event_type in &events.types {
        let owner = index(plan, &event_type.name)?;
        for listener in &event_type.listeners {
            let method = listener_method(events, event_type, listener)?;
            let _ = writeln!(
                out,
                "      events.listen({}, (value) => c.p{owner}.{}(__yon_read_{}(value)), {}, {});",
                dart_literal(&listener_options(listener)),
                listener.method,
                payload_type(method),
                listener.replay,
                dart_literal(&format!("{}.{}", event_type.name, listener.method))
            );
        }
    }
    Ok(())
}

fn dart_lifecycle(out: &mut String, plan: &Plan, events: &EventModule) -> Result<(), String> {
    for event_type in &events.types {
        let has_lifecycle = event_type.post_construct.is_some() || event_type.pre_destroy.is_some();
        if !has_lifecycle {
            continue;
        }
        let owner = index(plan, &event_type.name)?;
        let _ = writeln!(out, "          case {owner}:");
        if let Some(method) = &event_type.post_construct {
            let _ = writeln!(out, "            c.p{owner}.{method}();");
        }
        if let Some(method) = &event_type.pre_destroy {
            let _ = writeln!(
                out,
                "            events.destroyers.add(() => c.p{owner}.{method}());"
            );
        }
    }
    Ok(())
}

fn dart_literal(json: &str) -> String {
    let escaped = json
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('$', "\\$");
    format!("'{escaped}'")
}

fn rust(plan: &Plan, events: &EventModule) -> Result<String, String> {
    let mut out =
        String::from("#[derive(Default)]\nstruct __EventContainer { order: Vec<usize>,\n");
    for (i, provider) in plan.providers.iter().enumerate() {
        let _ = writeln!(out, "p{i}: Option<std::sync::Arc<{}>>,", provider.name);
    }
    out.push_str("}\nimpl __EventContainer {\n");
    for (i, provider) in plan.providers.iter().enumerate() {
        rust_provider_getter(&mut out, plan, i, provider)?;
    }
    out.push_str("}\nstruct Handler;\n#[allow(non_snake_case)]\nimpl Handler {\n");
    for route in &plan.routes {
        rust_route(&mut out, plan, events, route)?;
    }
    if !events.topic_listeners().is_empty() {
        let _ = writeln!(
            out,
            "fn __yon_deliver(request: &__YonFrame) -> __YonAnswer {{ let mut events = __YonEvents::delivery(&request.body(), &[{}]); let mut c = __EventContainer::default();",
            topic_arguments(events)
        );
        rust_setup(&mut out, plan, events)?;
        out.push_str("events.receive_delivery(); events.drain(); __YonAnswer::empty(204) }\n");
    }
    out.push_str("}\n");
    Ok(out)
}

fn rust_setup(out: &mut String, plan: &Plan, events: &EventModule) -> Result<(), String> {
    for i in eager(plan, events)? {
        let _ = writeln!(out, "c.get{i}();");
    }
    rust_listeners(out, plan, events)?;
    out.push_str("for index in c.order.clone() { match index {\n");
    for event_type in &events.types {
        let owner = index(plan, &event_type.name)?;
        let _ = writeln!(out, "{owner} => {{");
        rust_lifecycle(out, events, event_type, owner);
        out.push_str("}\n");
    }
    out.push_str("_ => {} } }\n");
    Ok(())
}

fn rust_provider_getter(
    out: &mut String,
    plan: &Plan,
    i: usize,
    provider: &crate::yon::plan::Provider,
) -> Result<(), String> {
    let _ = writeln!(
        out,
        "fn get{i}(&mut self) -> std::sync::Arc<{}> {{ if let Some(value) = &self.p{i} {{ return value.clone(); }}",
        provider.name
    );
    let mut arguments = Vec::new();
    for (argument, dependency) in provider.dependencies.iter().enumerate() {
        let dependency = index(plan, dependency)?;
        let _ = writeln!(out, "let d{argument} = self.get{dependency}();");
        arguments.push(format!("d{argument}"));
    }
    let args = arguments.join(",");
    let _ = writeln!(
        out,
        "let value = std::sync::Arc::new({}::new({args})); self.p{i} = Some(value.clone()); self.order.push({i}); value }}",
        provider.name
    );
    Ok(())
}

fn rust_route(
    out: &mut String,
    plan: &Plan,
    events: &EventModule,
    route: &Route,
) -> Result<(), String> {
    let controller = index(plan, &plan.controller)?;
    let _ = writeln!(
        out,
        "fn {}(request: &__YonFrame) -> __YonAnswer {{ let mut events = __YonEvents::new(&[{}]); let mut c = __EventContainer::default();",
        route.verb,
        topic_arguments(events)
    );
    rust_setup(out, plan, events)?;
    let request = request_argument(route, "request");
    let controller_value = RouteCalls::borrowed(route, &format!("c.get{controller}()"));
    let mut errors = Vec::new();
    for (index, _) in RouteCalls::error_owners(plan) {
        errors.push(format!("&c.get{index}()"));
    }
    let invocation = RouteCalls::invocation_errors(route, &controller_value, ".", request, &errors);
    let _ = writeln!(
        out,
        "events.drain(); let result = {invocation}; events.drain(); yon_response(result) }}"
    );
    Ok(())
}

fn returns_result(events: &EventModule, event_type: &EventType, method_name: &str) -> bool {
    events
        .methods
        .iter()
        .find(|method| method.owner == event_type.name && method.name == method_name)
        .is_some_and(|method| method.return_type.replace(' ', "").starts_with("Result<"))
}

fn rust_lifecycle(out: &mut String, events: &EventModule, event_type: &EventType, owner: usize) {
    if let Some(method) = &event_type.post_construct {
        if returns_result(events, event_type, method) {
            let _ = writeln!(
                out,
                "assert!(!c.get{owner}().{method}().is_err(), \"Yon lifecycle initialization failed.\");"
            );
        } else {
            let _ = writeln!(out, "c.get{owner}().{method}();");
        }
    }
    if let Some(method) = &event_type.pre_destroy {
        let body = if returns_result(events, event_type, method) {
            format!(
                "if owner.{method}().is_err() {{ eprintln!(\"Yon local lifecycle cleanup failed.\"); }}"
            )
        } else {
            format!("owner.{method}();")
        };
        let _ = writeln!(
            out,
            "let owner = c.get{owner}(); events.destroyers.push(Box::new(move || {{ {body} }}));"
        );
    }
}

fn rust_listeners(out: &mut String, plan: &Plan, events: &EventModule) -> Result<(), String> {
    for event_type in &events.types {
        let owner = index(plan, &event_type.name)?;
        for listener in &event_type.listeners {
            let method = listener_method(events, event_type, listener)?;
            let body = rust_listener_body(method, listener);
            let _ = writeln!(
                out,
                "{{ let owner = c.get{owner}(); events.listen({}, move |value| {{ {body} }}, {}, {}); }}",
                literal(&listener_options(listener), false),
                listener.replay,
                listener_key(event_type, listener)
            );
        }
    }
    Ok(())
}

fn rust_listener_body(method: &EventMethod, listener: &EventListener) -> String {
    let payload = method.parameters[0].type_source.replace(' ', "");
    let argument = match payload.strip_prefix('&') {
        Some(model) => format!("&<{model} as YonRead>::read(&value)"),
        None => String::from("YonRead::read(&value)"),
    };
    let call = format!("owner.{}({argument})", listener.method);
    let return_type = method.return_type.replace(' ', "");
    if return_type.is_empty() || return_type == "()" {
        return format!("{call}; Json::Null");
    }
    if return_type.starts_with("Result<") {
        return format!(
            "match {call} {{ Ok(value) => Json::from(value), Err(_) => {{ eprintln!(\"Yon local event listener failed.\"); Json::Null }} }}"
        );
    }
    format!("Json::from({call})")
}

fn go(plan: &Plan, events: &EventModule) -> Result<String, String> {
    let mut out = String::from("\ntype __yonEventContainer struct {\n\torder []int\n");
    for (i, provider) in plan.providers.iter().enumerate() {
        let _ = writeln!(out, "\tp{i} *{}", provider.name);
    }
    out.push_str("}\n");
    for (i, provider) in plan.providers.iter().enumerate() {
        go_provider_getter(&mut out, plan, i, provider)?;
    }
    out.push_str("\ntype Handler struct{}\n");
    for route in &plan.routes {
        go_route(&mut out, plan, events, route)?;
    }
    if !events.topic_listeners().is_empty() {
        let _ = writeln!(
            out,
            "\nfunc (Handler) YonDeliver(body string) {{\n\tvar delivery struct {{\n\t\tTopic     string          `json:\"topic\"`\n\t\tPayload   json.RawMessage `json:\"payload\"`\n\t\tListeners []string        `json:\"listeners\"`\n\t}}\n\tif json.Unmarshal([]byte(body), &delivery) != nil {{\n\t\treturn\n\t}}\n\tevents := __yonNewTopicEvents({}, append([]string{{}}, delivery.Listeners...))\n\tdefer events.close()\n\tc := &__yonEventContainer{{}}",
            go_topics(events)
        );
        go_setup(&mut out, plan, events)?;
        out.push_str(
            "\t__yonEventReceive(delivery.Topic, string(delivery.Payload))\n\tevents.drain()\n}\n",
        );
    }
    Ok(out)
}

fn go_topics(events: &EventModule) -> String {
    let names: Vec<String> = events.topics.iter().map(|name| quote(name)).collect();
    format!("[]string{{{}}}", names.join(", "))
}

fn go_setup(out: &mut String, plan: &Plan, events: &EventModule) -> Result<(), String> {
    for i in eager(plan, events)? {
        let _ = writeln!(out, "\tc.get{i}()");
    }
    go_listeners(out, plan, events)?;
    out.push_str("\tfor _, index := range append([]int{}, c.order...) {\n\t\tswitch index {\n");
    for event_type in &events.types {
        let owner = index(plan, &event_type.name)?;
        let _ = writeln!(out, "\t\tcase {owner}:");
        go_lifecycle(out, events, event_type, owner)?;
    }
    out.push_str("\t\t}\n\t}\n");
    Ok(())
}

fn go_provider_getter(
    out: &mut String,
    plan: &Plan,
    i: usize,
    provider: &crate::yon::plan::Provider,
) -> Result<(), String> {
    let name = &provider.name;
    let _ = writeln!(
        out,
        "\nfunc (container *__yonEventContainer) get{i}() *{name} {{\n\tif container.p{i} != nil {{\n\t\treturn container.p{i}\n\t}}\n\tvalue := &{name}{{}}"
    );
    for (injection, dependency) in provider.injections.iter().zip(&provider.dependencies) {
        let dependency = index(plan, dependency)?;
        let dereference = if injection.pointer { "" } else { "*" };
        let _ = writeln!(
            out,
            "\tvalue.{} = {dereference}container.get{dependency}()",
            injection.field
        );
    }
    let _ = writeln!(
        out,
        "\tcontainer.p{i} = value\n\tcontainer.order = append(container.order, {i})\n\treturn value\n}}"
    );
    Ok(())
}

fn go_route(
    out: &mut String,
    plan: &Plan,
    events: &EventModule,
    route: &Route,
) -> Result<(), String> {
    use crate::yon::go_container::{GoContainer, GoRouteResult};
    let controller = index(plan, &plan.controller)?;
    let result = GoContainer::dispatch_result(&events.source, &plan.controller, route)?;
    let _ = writeln!(
        out,
        "\nfunc (Handler) {}(request *yonRequest) *yonResponse {{\n\tevents := __yonNewTopicEvents({}, nil)\n\tdefer events.close()\n\tc := &__yonEventContainer{{}}",
        route.verb,
        go_topics(events)
    );
    go_setup(out, plan, events)?;
    out.push_str("\tevents.drain()\n");
    let mut errors = Vec::new();
    for (index, _) in RouteCalls::error_owners(plan) {
        errors.push(format!("c.get{index}()"));
    }
    let call = RouteCalls::invocation_errors(
        route,
        &format!("c.get{controller}()"),
        ".",
        request_argument(route, "request"),
        &errors,
    );
    let answer = match result {
        GoRouteResult::Nothing => format!("\t{call}\n\tevents.drain()\n\treturn yonAnswer(nil)"),
        GoRouteResult::Value => {
            format!("\tvalue := {call}\n\tevents.drain()\n\treturn yonAnswer(value)")
        }
        GoRouteResult::Failure => {
            format!("\tfailure := {call}\n\tevents.drain()\n\treturn yonAnswer(nil, failure)")
        }
        GoRouteResult::ValueAndFailure => format!(
            "\tvalue, failure := {call}\n\tevents.drain()\n\treturn yonAnswer(value, failure)"
        ),
    };
    let _ = writeln!(out, "{answer}\n}}");
    Ok(())
}

fn go_listeners(out: &mut String, plan: &Plan, events: &EventModule) -> Result<(), String> {
    for event_type in &events.types {
        let owner = index(plan, &event_type.name)?;
        for listener in &event_type.listeners {
            let method = listener_method(events, event_type, listener)?;
            let body = go_listener_body(method, listener)?;
            let _ = writeln!(
                out,
                "\t{{\n\t\towner := c.get{owner}()\n\t\tevents.listen({}, func(value any) any {{\n{body}\n\t\t}}, {}, {})\n\t}}",
                quote(&listener_options(listener)),
                listener.replay,
                quote(&format!("{}.{}", event_type.name, listener.method))
            );
        }
    }
    Ok(())
}

fn go_listener_body(method: &EventMethod, listener: &EventListener) -> Result<String, String> {
    use crate::yon::go_container::GoRouteResult;
    let call = format!(
        "owner.{}(__yonEventRead[{}](value))",
        listener.method,
        payload_type(method)
    );
    let body = match GoRouteResult::of(&method.return_type)? {
        GoRouteResult::Nothing => format!("\t\t\t{call}\n\t\t\treturn nil"),
        GoRouteResult::Value => format!("\t\t\treturn {call}"),
        GoRouteResult::Failure => format!(
            "\t\t\tif {call} != nil {{\n\t\t\t\t__yonListenerFailed()\n\t\t\t}}\n\t\t\treturn nil"
        ),
        GoRouteResult::ValueAndFailure => format!(
            "\t\t\tresult, failure := {call}\n\t\t\tif failure != nil {{\n\t\t\t\t__yonListenerFailed()\n\t\t\t\treturn nil\n\t\t\t}}\n\t\t\treturn result"
        ),
    };
    Ok(body)
}

fn go_lifecycle(
    out: &mut String,
    events: &EventModule,
    event_type: &EventType,
    owner: usize,
) -> Result<(), String> {
    use crate::yon::go_container::GoRouteResult;
    if let Some(method) = &event_type.post_construct {
        let call = format!("c.get{owner}().{method}()");
        let statement = match GoRouteResult::of(&lifecycle_results(events, event_type, method))? {
            GoRouteResult::Nothing | GoRouteResult::Value => format!("\t\t\t{call}"),
            GoRouteResult::Failure => format!(
                "\t\t\tif {call} != nil {{\n\t\t\t\tpanic(\"Yon lifecycle initialization failed.\")\n\t\t\t}}"
            ),
            GoRouteResult::ValueAndFailure => format!(
                "\t\t\tif _, failure := {call}; failure != nil {{\n\t\t\t\tpanic(\"Yon lifecycle initialization failed.\")\n\t\t\t}}"
            ),
        };
        let _ = writeln!(out, "{statement}");
    }
    if let Some(method) = &event_type.pre_destroy {
        let call = format!("owner.{method}()");
        let statement = match GoRouteResult::of(&lifecycle_results(events, event_type, method))? {
            GoRouteResult::Nothing | GoRouteResult::Value => format!("\t\t\t\t\t{call}"),
            GoRouteResult::Failure => format!(
                "\t\t\t\t\tif {call} != nil {{\n\t\t\t\t\t\t__yonCleanupFailed()\n\t\t\t\t\t}}"
            ),
            GoRouteResult::ValueAndFailure => format!(
                "\t\t\t\t\tif _, failure := {call}; failure != nil {{\n\t\t\t\t\t\t__yonCleanupFailed()\n\t\t\t\t\t}}"
            ),
        };
        let _ = writeln!(
            out,
            "\t\t\t{{\n\t\t\t\towner := c.get{owner}()\n\t\t\t\tevents.destroyers = append(events.destroyers, func() {{\n{statement}\n\t\t\t\t}})\n\t\t\t}}"
        );
    }
    Ok(())
}

fn lifecycle_results(events: &EventModule, event_type: &EventType, method_name: &str) -> String {
    events
        .methods
        .iter()
        .find(|method| method.owner == event_type.name && method.name == method_name)
        .map_or_else(String::new, |method| method.return_type.clone())
}

fn literal(json: &str, kotlin: bool) -> String {
    let mut escaped = json.replace('\\', "\\\\").replace('"', "\\\"");
    if kotlin {
        escaped = escaped.replace('$', "\\$");
    }
    format!("\"{escaped}\"")
}

fn php_literal(json: &str) -> String {
    let escaped = json.replace('\\', "\\\\").replace('\'', "\\'");
    format!("'{escaped}'")
}

pub(super) fn listener_options(listener: &EventListener) -> String {
    let options = serde_json::json!({
        "events": listener.events,
        "order": listener.order,
        "condition": listener.condition,
        "async": listener.asynchronous,
        "id": listener.id,
        "models": listener.models,
        "returns": listener.returns,
    });
    options.to_string()
}

fn payload_type(method: &EventMethod) -> String {
    method.parameters[0]
        .type_source
        .trim()
        .trim_start_matches('&')
        .to_owned()
}
