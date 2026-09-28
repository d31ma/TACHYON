use super::ApiBaselineCheck;
use serde_json::{Value, json};
use std::path::PathBuf;

fn api() -> Value {
    json!({
        "routes": [{
            "route": "/orders",
            "methods": {
                "POST": {
                    "request": {
                        "query": { "page?": { "type": "integer" }, "store": { "type": "string" } },
                        "body": { "$ref": "#/models/Order" }
                    },
                    "status": 201,
                    "ok": { "type": "integer" }
                },
                "GET": { "request": {}, "status": 200 }
            }
        }],
        "models": {
            "Order": {
                "type": "object",
                "properties": { "total": { "type": "integer" }, "note": { "type": "string" } },
                "required": ["total"]
            },
            "Code": { "type": "string" }
        }
    })
}

fn breaks(change: impl FnOnce(&mut Value), version: &str) -> Vec<String> {
    let root = tempfile::tempdir().expect("temporary project");
    std::fs::create_dir_all(root.path().join("server")).expect("server folder");
    std::fs::create_dir_all(root.path().join("client")).expect("client folder");
    let mut baseline = api();
    baseline["application_version"] = json!("1.4.0");
    std::fs::write(
        root.path().join("server/api-baseline.json"),
        baseline.to_string(),
    )
    .expect("baseline");
    let manifest = json!({ "tac": { "application": { "version": version } } });
    std::fs::write(
        root.path().join("client/manifest.json"),
        manifest.to_string(),
    )
    .expect("manifest");
    let mut current = api();
    change(&mut current);
    let files = vec![(PathBuf::from("api.json"), current.to_string().into_bytes())];
    ApiBaselineCheck::verify(root.path(), &files)
        .err()
        .map(|failure| {
            failure
                .diagnostics()
                .iter()
                .map(|problem| format!("{} {}", problem.code, problem.message))
                .collect()
        })
        .unwrap_or_default()
}

fn assert_break(change: impl FnOnce(&mut Value), part: &str) {
    let found = breaks(change, "1.5.0");
    assert!(
        found
            .iter()
            .any(|problem| problem.starts_with("TY1338") && problem.contains(part)),
        "{part}: {found:?}"
    );
}

fn assert_allowed(change: impl FnOnce(&mut Value)) {
    let found = breaks(change, "1.5.0");
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn removed_routes_methods_models_and_fields_break_the_api() {
    assert_break(
        |api| api["routes"] = json!([]),
        "the route '/orders' was removed",
    );
    assert_break(
        |api| {
            api["routes"][0]["methods"]
                .as_object_mut()
                .expect("methods")
                .remove("GET");
        },
        "GET /orders was removed",
    );
    assert_break(
        |api| {
            api["models"]
                .as_object_mut()
                .expect("models")
                .remove("Code");
        },
        "the model Code was removed",
    );
    assert_break(
        |api| {
            api["models"]["Order"]["properties"]
                .as_object_mut()
                .expect("fields")
                .remove("note");
        },
        "Order.note was removed or changed its type",
    );
    assert_break(
        |api| {
            api["routes"][0]["methods"]["POST"]["request"]["query"]
                .as_object_mut()
                .expect("query")
                .remove("store");
        },
        "no longer reads the query field 'store'",
    );
}

#[test]
fn narrowed_types_new_required_fields_and_status_changes_break_the_api() {
    assert_break(
        |api| {
            api["routes"][0]["methods"]["POST"]["request"]["query"]["page?"] =
                json!({ "type": "string" });
        },
        "narrowed the query field 'page'",
    );
    assert_break(
        |api| {
            api["routes"][0]["methods"]["POST"]["request"]["query"]["limit"] =
                json!({ "type": "integer" });
        },
        "needs the new query field 'limit'",
    );
    assert_break(
        |api| api["models"]["Order"]["required"] = json!(["total", "note"]),
        "Order.note is now required",
    );
    assert_break(
        |api| api["routes"][0]["methods"]["POST"]["status"] = json!(200),
        "answers with status 200 instead of 201",
    );
    assert_break(
        |api| api["routes"][0]["methods"]["POST"]["ok"] = json!({ "type": "number" }),
        "a value that callers did not expect",
    );
    assert_break(
        |api| api["models"]["Code"] = json!({ "type": "integer" }),
        "the model Code changed its type",
    );
}

#[test]
fn additions_and_widened_inputs_are_allowed() {
    assert_allowed(|_| {});
    assert_allowed(|api| {
        api["routes"][0]["methods"]["POST"]["request"]["query"]["sort?"] =
            json!({ "type": "string" });
    });
    assert_allowed(|api| {
        api["routes"][0]["methods"]["POST"]["request"]["query"]["page?"] =
            json!({ "type": "number" });
    });
    assert_allowed(|api| {
        api["routes"][0]["methods"]["POST"]["request"]["query"]["store?"] =
            json!({ "anyOf": [{ "type": "string" }, { "type": "null" }] });
    });
    assert_allowed(|api| {
        api["models"]["Order"]["properties"]["tax"] = json!({ "type": "integer" });
    });
    assert_allowed(|api| {
        api["routes"]
            .as_array_mut()
            .expect("routes")
            .push(json!({ "route": "/new", "methods": {} }));
    });
}

#[test]
fn a_new_major_version_or_a_missing_baseline_allows_breaks() {
    assert!(breaks(|api| api["routes"] = json!([]), "2.0.0").is_empty());
    let root = tempfile::tempdir().expect("temporary project");
    assert!(ApiBaselineCheck::verify(root.path(), &[]).is_ok());
}
