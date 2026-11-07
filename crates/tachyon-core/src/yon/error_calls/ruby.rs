use super::{ErrorCalls, Plan, RouteCalls};
use std::fmt::Write as _;

pub(super) fn functions(plan: &Plan) -> String {
    let parameters = ErrorCalls::parameters(plan, "rb");
    let mut source = format!(
        "module YonGenerated\n  def self.handle_error(error, request{parameters})\n    YonErrors.handle(error, [\n"
    );
    let mut candidates = Vec::new();
    for method in &plan.errors {
        let Some((index, _)) = RouteCalls::error_owners(plan)
            .into_iter()
            .find(|(_, name)| *name == method.owner)
        else {
            continue;
        };
        let request = if method.takes_request {
            ", YonHttp.request(request)"
        } else {
            ""
        };
        candidates.push(format!(
            "      [{}, {}, -> {{ handler{index}.{}(error{request}) }}]",
            method.error_type, method.status, method.method
        ));
    }
    let _ = writeln!(source, "{}", candidates.join(",\n"));
    source.push_str("    ])\n  end\nend\n");
    source
}
