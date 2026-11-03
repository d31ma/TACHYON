use super::{ErrorCalls, Plan, RouteCalls};
use std::fmt::Write as _;

pub(super) fn functions(plan: &Plan, typed: bool) -> String {
    let extension = if typed { "ts" } else { "js" };
    let parameters = ErrorCalls::parameters(plan, extension);
    let any = if typed { ": any" } else { "" };
    let list = if typed { ": any[]" } else { "" };
    let mut source = format!(
        "\nasync function __yon_handle_error__(error{any}, request{any}{parameters}) {{\n  if (!(error instanceof Error)) throw error;\n  const matches{list} = [];\n"
    );
    for method in &plan.errors {
        let Some((index, _)) = RouteCalls::error_owners(plan)
            .into_iter()
            .find(|(_, name)| *name == method.owner)
        else {
            continue;
        };
        let kind = &method.error_type;
        let request = match (method.takes_request, typed) {
            (false, _) => "",
            (true, false) => ", YonHttp.request(request)",
            (true, true) => ", (globalThis as any).YonHttp.request(request)",
        };
        let _ = writeln!(
            source,
            "  if (!({kind} === Error || {kind}.prototype instanceof Error)) throw new TypeError('HandlesError requires an Error type');\n  if (error instanceof {kind}) {{\n    let distance = 0;\n    let current = Object.getPrototypeOf(error);\n    while (current !== {kind}.prototype && current !== null) {{ distance += 1; current = Object.getPrototypeOf(current); }}\n    matches.push({{ distance, status: {}, call: () => handler{index}.{}(error{request}) }});\n  }}",
            method.status, method.method
        );
    }
    source.push_str("  matches.sort((left, right) => left.distance - right.distance);\n  if (matches.length === 0) throw error;\n  const selected = matches[0];\n  const result = await selected.call();\n  if (result instanceof Response) return result.status === 200 ? new Response(result.body, { status: selected.status, headers: result.headers }) : result;\n  return new Response(JSON.stringify(result), { status: selected.status, headers: { 'content-type': 'application/json' } });\n}\n");
    source
}
