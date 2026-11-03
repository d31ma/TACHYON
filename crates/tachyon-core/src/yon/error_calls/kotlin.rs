use super::{ErrorCalls, Plan, RouteCalls};
use std::fmt::Write as _;

pub(super) fn functions(plan: &Plan) -> String {
    let parameters = ErrorCalls::parameters(plan, "kt");
    let mut source = format!(
        "\nprivate object __YonErrors {{\n    fun handle(error: Exception, request: __YonFrame{parameters}): Any? {{\n        var selected = -1\n        var nearest = Int.MAX_VALUE\n"
    );
    for (position, method) in plan.errors.iter().enumerate() {
        let kind = &method.error_type;
        let _ = writeln!(
            source,
            "        if (error is {kind}) {{ val distance = distance(error.javaClass, {kind}::class.java); if (distance < nearest) {{ selected = {position}; nearest = distance }} }}"
        );
    }
    source.push_str("        return when (selected) {\n");
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
            "            {position} -> answer(handler{index}.{}(error as {}{request}), {})",
            method.method, method.error_type, method.status
        );
    }
    source.push_str("            else -> throw error\n        }\n    }\n    private fun distance(actual: Class<*>, wanted: Class<*>): Int { var current: Class<*>? = actual; var distance = 0; while (current != null && current != wanted) { current = current.superclass; distance++ }; return distance }\n    private fun answer(result: Any?, status: Int): Any? {\n        if (result is __YonAnswer) {\n            val original = (Json.parse(result.envelope()) as Map<*, *>)[\"status\"] as Number\n            return if (original.toInt() == 200) result.status(status) else result\n        }\n        return __YonAnswer.json(Json.stringify(result)).status(status)\n    }\n}\n");
    source
}
