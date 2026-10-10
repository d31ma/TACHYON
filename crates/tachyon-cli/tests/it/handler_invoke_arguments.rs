#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

const CONTROLLER: &str = "server/controllers/ItemsController.js";

fn write(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().expect("fixture parent")).expect("fixture directory");
    fs::write(path, contents).expect("fixture source");
}

fn write_project(project: &Path) {
    write(
        &project.join("server/models/Item.js"),
        r"/**
 * @typedef {Object} Item
 * @property {string} name
 */
export {}
",
    );
    write(
        &project.join(CONTROLLER),
        r#"@RequestRoute('/items')
@Public
export class ItemsController {
  /**
   * @param {number} id
   * @param {number} limit
   * @param {string} trace
   * @param {string} session
   * @param {Item} item
   */
  @PostRoute('/{id}')
  post(@FromPath id, @FromQuery("limit") limit, @FromHeader("x-trace") trace, @FromCookie("session") session, @FromBody item) {
    return { id, limit, trace, session, item };
  }

  /** @param {Item} item */
  @PostRoute('/echo')
  postEcho(@FromBody item) {
    return { name: item.name.toUpperCase() };
  }
}
"#,
    );
}

fn invoke(project: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ty"))
        .args(["handler", "invoke", CONTROLLER])
        .arg("--project")
        .arg(project)
        .args(arguments)
        .output()
        .expect("the ty process should start")
}

fn response(output: &Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("protocol response JSON")
}

fn body(response: &serde_json::Value) -> serde_json::Value {
    let data = response["body"]["data"].as_str().expect("body data");
    serde_json::from_str(data).expect("JSON body")
}

#[test]
fn a_concrete_target_binds_path_query_header_cookie_and_body_arguments() {
    let project = tempfile::tempdir().expect("project");
    write_project(project.path());
    let output = invoke(
        project.path(),
        &[
            "--target",
            "/items/42?limit=3",
            "--method",
            "post",
            "--header",
            "x-trace=abc",
            "--header",
            "cookie=session=s1",
            "--body",
            r#"{"name":"Ada"}"#,
        ],
    );
    let response = response(&output);
    assert_eq!(response["status"], 200, "{response}");
    assert_eq!(
        body(&response),
        serde_json::json!({
            "id": 42,
            "limit": 3,
            "trace": "abc",
            "session": "s1",
            "item": { "name": "Ada" },
        })
    );
}

#[test]
fn a_static_route_binds_the_body_without_a_target() {
    let project = tempfile::tempdir().expect("project");
    write_project(project.path());
    let output = invoke(
        project.path(),
        &[
            "--route",
            "/items/echo",
            "--method",
            "post",
            "--body",
            r#"{"name":"hi"}"#,
        ],
    );
    let response = response(&output);
    assert_eq!(response["status"], 200, "{response}");
    assert_eq!(body(&response), serde_json::json!({ "name": "HI" }));
}

#[test]
fn an_invalid_body_gets_the_server_400_answer() {
    let project = tempfile::tempdir().expect("project");
    write_project(project.path());
    let output = invoke(
        project.path(),
        &[
            "--route",
            "/items/echo",
            "--method",
            "post",
            "--body",
            r#"{"name":7}"#,
        ],
    );
    let response = response(&output);
    assert_eq!(response["status"], 400, "{response}");
    let refusal = body(&response);
    assert_eq!(refusal["part"], "body", "{refusal}");
    assert!(refusal["error"].is_string(), "{refusal}");
}

#[test]
fn a_target_that_no_route_answers_is_an_invalid_request() {
    let project = tempfile::tempdir().expect("project");
    write_project(project.path());
    let output = invoke(
        project.path(),
        &["--target", "/orders/1", "--method", "post"],
    );
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("TY2005"), "{stderr}");
}
