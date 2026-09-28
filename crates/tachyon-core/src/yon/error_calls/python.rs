use super::{ErrorCalls, Plan, RouteCalls};
use std::fmt::Write as _;

pub(super) fn functions(plan: &Plan) -> String {
    let parameters = ErrorCalls::parameters(plan, "py");
    let mut source = format!(
        "\nimport inspect as __yon_error_inspect\nimport json as __yon_error_json\n\nasync def __yon_handle_error__(error, request{parameters}):\n    if not isinstance(error, Exception):\n        raise error\n    matches = []\n"
    );
    for method in &plan.errors {
        let Some((index, _)) = RouteCalls::error_owners(plan)
            .into_iter()
            .find(|(_, name)| *name == method.owner)
        else {
            continue;
        };
        let kind = &method.error_type;
        let request = if method.takes_request {
            ", YonHttp.request(request)"
        } else {
            ""
        };
        let _ = writeln!(
            source,
            "    if not isinstance({kind}, type) or not issubclass({kind}, Exception):\n        raise TypeError('HandlesError requires an Exception type')\n    if isinstance(error, {kind}):\n        distance = type(error).__mro__.index({kind})\n        matches.append((distance, {}, lambda: handler{index}.{}(error{request})))",
            method.status, method.method
        );
    }
    source.push_str("    if not matches:\n        raise error\n    matches.sort(key=lambda candidate: candidate[0])\n    selected = matches[0]\n    result = selected[2]()\n    if __yon_error_inspect.isawaitable(result):\n        result = await result\n    if YonHttp.is_answer(result):\n        return (selected[1] if result[0] == 200 else result[0], result[1], result[2])\n    return (selected[1], {'content-type': 'application/json'}, __yon_error_json.dumps(YonModels.plain(result), ensure_ascii=False, separators=(',', ':'), allow_nan=False))\n");
    source
}
