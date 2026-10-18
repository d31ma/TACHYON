#![allow(clippy::expect_used)]

use crate::support::json_http::JsonHttp;
use std::path::Path;
use std::process::{Command, Stdio};

struct CheckedOrders;

impl CheckedOrders {
    const FILES: [(&'static str, &'static str); 3] = [
        (
            "server/models/OrderInput.java",
            "@Checked\nclass OrderInput {\n    @NotEmpty\n    @Length({\"min\":2,\"max\":8})\n    String name;\n    @Required\n    @Min(1)\n    @Max(9)\n    Object level;\n}\n",
        ),
        (
            "server/services/OrderService.java",
            "@Service\nclass OrderService {\n    Object accept(@Check(OrderInput.class) Object order) { return order; }\n}\n",
        ),
        (
            "server/controllers/OrdersController.java",
            "@RequestRoute(\"/orders\")\n@Public\nclass OrdersController {\n    @Needs\n    final OrderService orders;\n    @PostRoute\n    Object postOrders(@FromBody Object order) {\n        return orders.accept(order);\n    }\n}\n",
        ),
    ];

    const ACCEPTED: [(&'static str, &'static str); 6] = [
        ("every rule holds", r#"{"name":"ab","level":3}"#),
        ("Length accepts the minimum", r#"{"name":"ab","level":5}"#),
        (
            "Length accepts the maximum",
            r#"{"name":"abcdefgh","level":5}"#,
        ),
        ("Min accepts the bound", r#"{"name":"order","level":1}"#),
        ("Max accepts the bound", r#"{"name":"order","level":9}"#),
        (
            "unknown fields stay",
            r#"{"name":"order","level":2,"sku":"A-1"}"#,
        ),
    ];

    const REJECTED: [(&'static str, &'static str, &'static str); 10] = [
        (
            "Required refuses a missing value",
            r#"{"name":"order"}"#,
            r#"[{"field":"level","rule":"Required"}]"#,
        ),
        (
            "Required refuses null",
            r#"{"name":"order","level":null}"#,
            r#"[{"field":"level","rule":"Required"}]"#,
        ),
        (
            "NotEmpty refuses a missing value",
            r#"{"level":3}"#,
            r#"[{"field":"name","rule":"NotEmpty"}]"#,
        ),
        (
            "NotEmpty refuses an empty text",
            r#"{"name":"","level":3}"#,
            r#"[{"field":"name","rule":"NotEmpty"},{"field":"name","rule":"Length"}]"#,
        ),
        (
            "NotEmpty refuses blank text",
            r#"{"name":"   ","level":3}"#,
            r#"[{"field":"name","rule":"NotEmpty"}]"#,
        ),
        (
            "Length refuses a short text",
            r#"{"name":"a","level":3}"#,
            r#"[{"field":"name","rule":"Length"}]"#,
        ),
        (
            "Length refuses a long text",
            r#"{"name":"abcdefghi","level":3}"#,
            r#"[{"field":"name","rule":"Length"}]"#,
        ),
        (
            "Min refuses a smaller number",
            r#"{"name":"order","level":0}"#,
            r#"[{"field":"level","rule":"Min"}]"#,
        ),
        (
            "Max refuses a larger number",
            r#"{"name":"order","level":10}"#,
            r#"[{"field":"level","rule":"Max"}]"#,
        ),
        (
            "Min refuses a text",
            r#"{"name":"order","level":"3"}"#,
            r#"[{"field":"level","rule":"Min"},{"field":"level","rule":"Max"}]"#,
        ),
    ];

    fn java_available() -> bool {
        let found = ["java", "javac"].iter().all(|program| {
            Command::new(program)
                .arg("-version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok_and(|status| status.success())
        });
        if !found {
            assert!(std::env::var_os("CI").is_none(), "java is required in CI");
            eprintln!("skipping Java checked models: java or javac is unavailable");
        }
        found
    }

    fn write(root: &Path) {
        for (relative, source) in Self::FILES {
            let path = root.join(relative);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("directories");
            std::fs::write(path, source).expect("source");
        }
    }

    fn all_bodies() -> Vec<&'static str> {
        let mut bodies = Vec::new();
        for (_, body) in Self::ACCEPTED {
            bodies.push(body);
        }
        for (_, body, _) in Self::REJECTED {
            bodies.push(body);
        }
        bodies
    }

    fn split(response: &str) -> (&str, &str) {
        let (head, body) = response.split_once("\r\n\r\n").expect("HTTP response");
        let status_line = head.lines().next().expect("status line");
        (status_line, body)
    }

    fn check_accepted(name: &str, sent: &str, response: &str) {
        let (status_line, body) = Self::split(response);
        assert_eq!(status_line, "HTTP/1.1 200 OK", "{name}: {response}");
        assert!(
            response.contains("content-type: application/json"),
            "{name}: {response}"
        );
        let answered: serde_json::Value = serde_json::from_str(body).expect("JSON answer");
        let expected: serde_json::Value = serde_json::from_str(sent).expect("JSON request");
        assert_eq!(answered, expected, "{name}");
    }

    fn check_rejected(name: &str, errors: &str, response: &str) {
        let (status_line, body) = Self::split(response);
        assert_eq!(
            status_line, "HTTP/1.1 422 Unprocessable Entity",
            "{name}: {response}"
        );
        assert!(
            response.contains("content-type: application/problem+json"),
            "{name}: {response}"
        );
        let answered: serde_json::Value =
            serde_json::from_str(body).unwrap_or_else(|_| panic!("{name}: {body}"));
        let reference = answered["reference"].as_str().unwrap_or_default();
        let reference_is_plain = reference
            .chars()
            .all(|character| character.is_ascii_alphanumeric());
        assert!(
            !reference.is_empty() && reference_is_plain,
            "{name}: {body}"
        );
        assert!(
            response.contains(&format!("x-tachyon-request-id: {reference}")),
            "{name}: {response}"
        );
        let expected = format!(
            r#"{{"type":"about:blank","title":"Invalid input","status":422,"errors":{errors},"truncated":false,"reference":"{reference}"}}"#
        );
        assert_eq!(body, expected, "{name}");
    }
}

#[tokio::test]
async fn a_java_service_checks_a_model_from_server_models_over_http() {
    if !CheckedOrders::java_available() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    CheckedOrders::write(root.path());
    let bodies = CheckedOrders::all_bodies();
    let responses = JsonHttp::post_all(root.path(), "/orders", &bodies).await;
    let (accepted, rejected) = responses.split_at(CheckedOrders::ACCEPTED.len());
    for ((name, sent), response) in CheckedOrders::ACCEPTED.iter().zip(accepted) {
        CheckedOrders::check_accepted(name, sent, response);
    }
    for ((name, _, errors), response) in CheckedOrders::REJECTED.iter().zip(rejected) {
        CheckedOrders::check_rejected(name, errors, response);
    }
}
