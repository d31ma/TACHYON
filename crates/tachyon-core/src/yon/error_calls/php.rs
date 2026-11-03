use super::{ErrorCalls, Plan, RouteCalls};
use std::fmt::Write as _;

pub(super) fn functions(plan: &Plan) -> String {
    let parameters = ErrorCalls::parameters(plan, "php");
    let mut source = format!(
        "\nfunction __yon_handle_error__(\\Throwable $error, $request{parameters}) {{\n    $matches = [];\n"
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
            ", $request"
        } else {
            ""
        };
        let _ = writeln!(
            source,
            "    if (!is_a({kind}::class, \\Throwable::class, true)) throw new \\TypeError('HandlesError requires a Throwable type');\n    if ($error instanceof {kind}) {{\n        $distance = 0;\n        $current = get_class($error);\n        while ($current !== {kind}::class && $current !== false) {{ $distance++; $current = get_parent_class($current); }}\n        $matches[] = [$distance, {}, fn() => $handler{index}->{}($error{request})];\n    }}",
            method.status, method.method
        );
    }
    source.push_str("    if (!$matches) throw $error;\n    usort($matches, fn($left, $right) => $left[0] <=> $right[0]);\n    $selected = $matches[0];\n    $result = $selected[2]();\n    if ($result instanceof __YonAnswer) {\n        $envelope = json_decode($result->envelope(), true, 64, JSON_THROW_ON_ERROR);\n        return $envelope['status'] === 200 ? $result->status($selected[1]) : $result;\n    }\n    return __YonAnswer::json(json_encode($result, JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE))->status($selected[1]);\n}\n");
    source
}
