use serde_json::{Value, json};

struct Condition;

impl Condition {
    const MAX_SOURCE_BYTES: usize = 512;
    const MAX_DEPTH: usize = 16;
    const MAX_PATH_SEGMENTS: usize = 8;
}

pub(crate) fn compile(source: &str) -> Result<Value, String> {
    if source.trim().is_empty() || source.len() > Condition::MAX_SOURCE_BYTES {
        return Err(String::from("condition must be 1 through 512 bytes"));
    }
    let expression = crate::template::Expression::parse(source, "condition", 0)
        .map_err(|_| String::from("condition is not a valid bounded expression"))?;
    let tree: Value = serde_json::from_str(&expression.to_client_json())
        .map_err(|_| String::from("condition could not be encoded"))?;
    compile_test(&tree, 0)
}

fn check_depth(depth: usize) -> Result<(), String> {
    if depth > Condition::MAX_DEPTH {
        return Err(String::from("condition nests too deeply"));
    }
    Ok(())
}

fn compile_test(node: &Value, depth: usize) -> Result<Value, String> {
    check_depth(depth)?;
    let next = depth + 1;
    match node["k"].as_str() {
        Some("cmp") => Ok(json!({
            "k": "cmp",
            "op": node["op"],
            "l": compile_operand(&node["l"], next)?,
            "r": compile_operand(&node["r"], next)?,
        })),
        Some("log") => Ok(json!({
            "k": node["op"],
            "l": compile_test(&node["l"], next)?,
            "r": compile_test(&node["r"], next)?,
        })),
        Some("not") => Ok(json!({ "k": "not", "e": compile_test(&node["e"], next)? })),
        _ => Ok(json!({ "k": "truthy", "e": compile_operand(node, next)? })),
    }
}

fn compile_operand(node: &Value, depth: usize) -> Result<Value, String> {
    check_depth(depth)?;
    match node["k"].as_str() {
        Some("lit") => Ok(json!({ "k": "lit", "v": node["v"] })),
        Some("id") => compile_name(node),
        Some("get" | "idx") => compile_path_step(node, depth),
        _ => Err(String::from(
            "condition allows comparisons, and, or, not, payload paths and literals only",
        )),
    }
}

fn compile_name(node: &Value) -> Result<Value, String> {
    match node["n"].as_str() {
        Some("value") => Ok(json!({ "k": "value", "path": [] })),
        Some("event") => Ok(json!({ "k": "event" })),
        _ => Err(String::from(
            "condition reads only `value` (the payload) and `event` (the name)",
        )),
    }
}

fn compile_path_step(node: &Value, depth: usize) -> Result<Value, String> {
    let mut inner = compile_operand(&node["o"], depth + 1)?;
    if inner["k"] != "value" {
        return Err(String::from("condition paths start at `value`"));
    }
    let segment = if node["k"] == "get" {
        node["p"].clone()
    } else {
        node["i"].clone()
    };
    let Some(path) = inner["path"].as_array_mut() else {
        return Err(String::from("condition path is malformed"));
    };
    if path.len() >= Condition::MAX_PATH_SEGMENTS {
        return Err(String::from("condition paths have at most 8 segments"));
    }
    path.push(segment);
    Ok(inner)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn comparisons_paths_and_logic_compile_into_a_matcher() {
        let matcher =
            compile("value.status == 'paid' && value.lines[0].total >= 10 || !value.draft")
                .expect("matcher");
        assert_eq!(matcher["k"], "or");
        assert_eq!(matcher["l"]["k"], "and");
        assert_eq!(matcher["l"]["l"]["op"], "eq");
        assert_eq!(matcher["l"]["l"]["l"]["path"], json!(["status"]));
        assert_eq!(matcher["l"]["r"]["l"]["path"], json!(["lines", 0, "total"]));
        assert_eq!(matcher["r"]["k"], "not");
        assert_eq!(matcher["r"]["e"]["k"], "truthy");
        assert_eq!(
            compile("event == 'cart.total'").expect("event")["l"]["k"],
            "event"
        );
    }

    #[test]
    fn calls_arithmetic_and_unknown_roots_are_refused() {
        assert!(compile("value.total()").is_err());
        assert!(compile("value.total + 1 > 2").is_err());
        assert!(compile("other.total > 2").is_err());
        assert!(compile("").is_err());
    }
}
