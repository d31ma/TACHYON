use super::{CppYonController, CppYonErrors, CppYonMiddleware, CppYonSocketCalls};
use crate::yon::plan::{ParameterSource, Route};

pub(crate) struct CppYonDispatcher;

impl CppYonDispatcher {
    const ROUTES: &'static str = "TachyonYonRoutes";
    const VOID: &'static str = "void";
    const START: &'static str = "#line 1 \"tachyon-yon-routes.cpp\"\n";

    pub(crate) fn source(controller: &CppYonController, errors: &CppYonErrors) -> String {
        let socket = controller
            .routes
            .iter()
            .find_map(|route| route.socket.as_ref().map(|methods| (route, methods)));
        let body = socket.map_or_else(
            || Self::routes(controller),
            |(route, methods)| CppYonSocketCalls::body(route, methods),
        );
        format!(
            "{start}class {routes} {{\npublic:\n    static tachyon::yon::YonAnswer answer(const tachyon::yon::YonFrame& frame) {{\n        tachyon::yon::YonInvocation invocation;\n        {class}& controller = tachyon::yon::YonNeeds::of<{class}>();\n        invocation.start();\n{body}    }}\n\nprivate:\n{handling}}};\n\nint main() {{\n    return tachyon::yon::YonMain::run<{routes}>();\n}}\n",
            start = Self::START,
            routes = Self::ROUTES,
            class = controller.file.class.name,
            handling = errors.handling(),
        )
    }

    fn routes(controller: &CppYonController) -> String {
        let branches: String = controller.routes.iter().map(Self::branch).collect();
        format!(
            "        std::string verb = frame.text(\"{field}\");\n        std::string route = frame.text(\"route\");\n        try {{\n{branches}        }} catch (...) {{\n            return handled(std::current_exception(), frame);\n        }}\n        return {unrouted};\n",
            field = CppYonMiddleware::field(&controller.path),
            unrouted = CppYonMiddleware::unrouted(&controller.path),
        )
    }

    fn branch(route: &Route) -> String {
        let call = format!("controller.{}({})", route.method, Self::arguments(route));
        let path = route
            .pattern
            .as_ref()
            .map(|pattern| format!(" && route == \"{pattern}\""))
            .unwrap_or_default();
        format!(
            "        if (verb == \"{}\"{path}) {{\n{}        }}\n",
            route.verb,
            Self::answer(route, &call)
        )
    }

    fn answer(route: &Route, call: &str) -> String {
        if route.streaming {
            return format!(
                "            return tachyon::yon::YonStream::send(frame, invocation, [&]() {{ return {call}; }});\n"
            );
        }
        let empty = format!(
            "            {call};\n            invocation.drain();\n            return tachyon::yon::YonAnswer::empty(204);\n"
        );
        let valued = format!(
            "            auto result = {call};\n            invocation.drain();\n            return tachyon::yon::YonAnswer::of(result);\n"
        );
        [valued, empty][usize::from(route.returns == Self::VOID)].clone()
    }

    fn arguments(route: &Route) -> String {
        let arguments: Vec<String> = route
            .parameters
            .iter()
            .enumerate()
            .map(|(index, parameter)| Self::argument(index, parameter))
            .collect();
        arguments.join(", ")
    }

    fn argument(index: usize, parameter: &crate::yon::plan::RouteParameter) -> String {
        let read = format!("frame.argument<{}>({index})", parameter.type_text);
        let request = String::from("tachyon::yon::YonRequest::of(frame)");
        [read, request][usize::from(parameter.source == ParameterSource::Request)].clone()
    }
}
