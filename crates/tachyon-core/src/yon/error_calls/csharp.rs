use super::{ErrorCalls, Plan, RouteCalls};
use std::fmt::Write as _;

pub(super) fn functions(plan: &Plan) -> String {
    let parameters = ErrorCalls::parameters(plan, "cs");
    let mut source = format!(
        "\nstatic class __YonErrors {{\n    internal static object Handle(System.Exception error, __YonFrame request{parameters}) {{\n        int selected = -1;\n        int nearest = int.MaxValue;\n"
    );
    for (position, method) in plan.errors.iter().enumerate() {
        let kind = &method.error_type;
        let _ = writeln!(
            source,
            "        if (error is {kind}) {{ int distance = Distance(error.GetType(), typeof({kind})); if (distance < nearest) {{ selected = {position}; nearest = distance; }} }}"
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
            ", YonHttp.Request(request)"
        } else {
            ""
        };
        let _ = writeln!(
            source,
            "            case {position}: return Answer(handler{index}.{}(({}) error{request}), {});",
            method.method, method.error_type, method.status
        );
    }
    source.push_str("            default: throw error;\n        }\n    }\n    static int Distance(System.Type actual, System.Type wanted) { int distance = 0; while (actual != null && actual != wanted) { actual = actual.BaseType; distance++; } return distance; }\n    static object Answer(object result, int status) {\n        if (result is System.Net.Http.HttpResponseMessage message) {\n            var response = YonHttp.Answer(message);\n            return (int)message.StatusCode == 200 ? response.Status(status) : response;\n        }\n        return __YonAnswer.Json(System.Text.Json.JsonSerializer.Serialize(YonModels.Answer(result))).Status(status);\n    }\n}\n");
    source
}
