use super::CppYonController;
use crate::tac_cpp::{CppLowering, CppPrelude, CppProgram};
use crate::yon::plan::Route;

pub(crate) struct CppYonProgram;

impl CppYonProgram {
    const RUNTIME: &'static str = include_str!("tachyon/yon.hpp");
    const ROUTES: &'static str = "TachyonYonRoutes";
    const VOID: &'static str = "void";
    const START: &'static str = "#line 1 \"tachyon-yon-routes.cpp\"\n";

    pub(crate) fn source(controller: &CppYonController, models: &CppProgram) -> String {
        format!(
            "{}{}{}{}{}",
            CppPrelude::json(),
            Self::RUNTIME,
            models.model_source(),
            Self::located_controller(controller),
            Self::dispatcher(controller)
        )
    }

    fn located_controller(controller: &CppYonController) -> String {
        let mut lowered = CppLowering::model(&controller.file, &controller.source);
        let friend = format!(" friend class {};", Self::ROUTES);
        lowered.insert_str(controller.file.class.body.start + 1, &friend);
        CppProgram::located(&controller.path, &lowered)
    }

    fn dispatcher(controller: &CppYonController) -> String {
        let branches: String = controller.routes.iter().map(Self::branch).collect();
        format!(
            "{start}class {routes} {{\npublic:\n    static tachyon::yon::YonAnswer answer(const tachyon::yon::YonFrame& frame) {{\n        {class} controller;\n        std::string verb = frame.text(\"method\");\n        std::string route = frame.text(\"route\");\n{branches}        return tachyon::yon::YonAnswer::empty(405);\n    }}\n}};\n\nint main() {{\n    return tachyon::yon::YonMain::run<{routes}>();\n}}\n",
            start = Self::START,
            routes = Self::ROUTES,
            class = controller.file.class.name,
        )
    }

    fn branch(route: &Route) -> String {
        let call = format!("controller.{}({})", route.method, Self::arguments(route));
        format!(
            "        if (verb == \"{}\" && route == \"{}\") {{\n{}        }}\n",
            route.verb,
            route.pattern.as_deref().unwrap_or_default(),
            Self::answer(route, &call)
        )
    }

    fn answer(route: &Route, call: &str) -> String {
        if route.returns == Self::VOID {
            return format!(
                "            {call};\n            return tachyon::yon::YonAnswer::empty(204);\n"
            );
        }
        format!("            return tachyon::yon::YonAnswer::of({call});\n")
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
