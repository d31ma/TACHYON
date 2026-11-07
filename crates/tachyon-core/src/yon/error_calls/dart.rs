use super::{ErrorCalls, Plan, RouteCalls};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

pub(super) fn functions(plan: &Plan, stripped: &str) -> String {
    let parameters = ErrorCalls::parameters(plan, "dart");
    let mut source = format!(
        "\nFuture<Object?> __yon_handle_error__(Object error, __YonFrame request{parameters}) async {{\n  if (error is! Exception && error is! Error) throw error;\n"
    );
    let parents = parents(stripped);
    let mut methods: Vec<_> = plan.errors.iter().collect();
    methods.sort_by_key(|method| std::cmp::Reverse(depth(&parents, &method.error_type)));
    for method in methods {
        let Some((index, _)) = RouteCalls::error_owners(plan)
            .into_iter()
            .find(|(_, name)| *name == method.owner)
        else {
            continue;
        };
        let request = if method.takes_request {
            ", _yonShelfRequest(request)"
        } else {
            ""
        };
        let _ = writeln!(
            source,
            "  if (error is {}) {{\n    final Object? result = await handler{index}.{}(error{request});\n    if (result is yon_shelf.Response) {{\n      final answer = await _yonShelfAnswer(result);\n      return result.statusCode == 200 ? answer.status({}) : answer;\n    }}\n    return __YonAnswer.json(yon_convert.jsonEncode(__yon_json(result))).status({});\n  }}",
            method.error_type, method.method, method.status, method.status
        );
    }
    source.push_str("  throw error;\n}\n");
    source
}

fn parents(source: &str) -> BTreeMap<String, String> {
    let mut parents = BTreeMap::new();
    let mask = crate::lexical::code_mask("dart", source);
    let mut code = source.as_bytes().to_vec();
    for (byte, visible) in code.iter_mut().zip(mask.code) {
        if !visible {
            *byte = b' ';
        }
    }
    let code = String::from_utf8(code).unwrap_or_default();
    let words: Vec<_> = code
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .filter(|word| !word.is_empty())
        .collect();
    for declaration in words.windows(4) {
        if declaration[0] == "class" && matches!(declaration[2], "extends" | "implements") {
            parents.insert(declaration[1].to_owned(), declaration[3].to_owned());
        }
    }
    parents
}

fn depth(parents: &BTreeMap<String, String>, name: &str) -> usize {
    let mut seen = BTreeSet::new();
    let mut current = name;
    while seen.insert(current) {
        let Some(parent) = parents.get(current) else {
            break;
        };
        current = parent;
    }
    seen.len()
}
