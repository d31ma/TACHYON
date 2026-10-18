use super::CppYonController;
use crate::yon::plan::Route;

pub(crate) struct CppYonDispatcher;

impl CppYonDispatcher {
    const ROUTES: &'static str = "TachyonYonRoutes";
    const VOID: &'static str = "void";
    const START: &'static str = "#line 1 \"tachyon-yon-routes.cpp\"\n";

    pub(crate) fn source(controller: &CppYonController) -> String {
        let branches: String = controller.routes.iter().map(Self::branch).collect();
        format!(
            "{start}class {routes} {{\npublic:\n    static tachyon::yon::YonAnswer answer(const tachyon::yon::YonFrame& frame) {{\n        tachyon::yon::YonInvocation invocation;\n        {class}& controller = tachyon::yon::YonNeeds::of<{class}>();\n        invocation.start();\n        std::string verb = frame.text(\"method\");\n        std::string route = frame.text(\"route\");\n{branches}        return tachyon::yon::YonAnswer::empty(405);\n    }}\n}};\n\nint main() {{\n    return tachyon::yon::YonMain::run<{routes}>();\n}}\n",
            start = Self::START,
            routes = Self::ROUTES,
            class = controller.file.class.name,
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
            .map(|(index, parameter)| format!("frame.argument<{}>({index})", parameter.type_text))
            .collect();
        arguments.join(", ")
    }
}
