use super::{ErrorCalls, Plan, RouteCalls};
use std::fmt::Write as _;

pub(super) fn functions(plan: &Plan) -> String {
    let parameters = ErrorCalls::parameters(plan, "java");
    let mut source = format!(
        "\nfinal class __YonErrors {{\n    static Object handle(Exception error, __YonFrame request{parameters}) throws Exception {{\n        int selected = -1;\n        int nearest = Integer.MAX_VALUE;\n"
    );
    for (position, method) in plan.errors.iter().enumerate() {
        let kind = &method.error_type;
        let _ = writeln!(
            source,
            "        if (error instanceof {kind}) {{ int distance = distance(error.getClass(), {kind}.class); if (distance < nearest) {{ selected = {position}; nearest = distance; }} }}"
        );
    }
    source.push_str("        switch (selected) {\n");
    for (position, method) in plan.errors.iter().enumerate() {
        let Some((index, _)) = RouteCalls::error_owners(plan)
            .into_iter()
            .find(|(_, name)| *name == method.owner)
        else {
            continue;
        };
        let request = if method.takes_request {
            ", Yon.exchange(request)"
        } else {
            ""
        };
        let _ = writeln!(
            source,
            "            case {position}: return answer(handler{index}.{}(({}) error{request}), {});",
            method.method, method.error_type, method.status
        );
    }
    source.push_str("            default: throw error;\n        }\n    }\n    static int distance(Class<?> actual, Class<?> wanted) { int distance = 0; while (actual != null && actual != wanted) { actual = actual.getSuperclass(); distance++; } return distance; }\n    static Object answer(Object result, int status) {\n        if (result instanceof __YonAnswer response) {\n            Object original = ((java.util.Map<?, ?>) Json.parse(response.envelope())).get(\"status\");\n            return original instanceof Number number && number.intValue() == 200 ? response.status(status) : response;\n        }\n        return __YonAnswer.json(Json.stringify(result)).status(status);\n    }\n}\n");
    source
}
