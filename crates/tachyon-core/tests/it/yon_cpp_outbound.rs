#![allow(clippy::expect_used)]

use super::yon_cpp::{CppProject, Exchange};
use std::path::Path;
use tachyon_core::{DevServer, DevServerOptions, ProjectDiscovery};

struct CppOutbound;

impl CppOutbound {
    const CLIENT_FILES: [(&'static str, &'static str); 4] = [
        (
            "server/models/Product.hpp",
            "#pragma once\n#include <string>\n\nstruct Product {\n    std::string sku;\n    int level = 0;\n};\n",
        ),
        (
            "server/setups/CatalogSetup.hpp",
            r#"#pragma once

class [[Setup]] CatalogSetup {
public:
    static BehaviorAdapters configure(const BehaviorExecution& execution) {
        BehaviorAdapters adapters;
        adapters.http["catalog"] = [partition = execution.partition](const tachyon::http::HttpRequestMessage& request) {
            tachyon::json::JsonObject sent;
            sent["method"] = request.method.toString();
            sent["uri"] = request.requestUri;
            sent["tenant"] = request.headers.contains("X-Tenant") ? request.headers.all().at("X-Tenant") : std::string();
            sent["body"] = request.content.has_value() ? tachyon::json::JsonNode::parse(request.content->readAsString()) : tachyon::json::JsonNode(nullptr);
            sent["partition"] = partition;
            if (request.method == tachyon::http::HttpMethod::Post) {
                return tachyon::json::JsonNode::parse(request.content->readAsString());
            }
            return tachyon::json::JsonNode(sent);
        };
        return adapters;
    }
};
"#,
        ),
        (
            "server/remotes/CatalogClient.hpp",
            r#"#pragma once
#include <string>
#include "../models/Product.hpp"

class [[HTTPClient("/api")]] CatalogClient {
public:
    [[GetRequest("/products/{id}")]]
    tachyon::json::JsonNode find([[PathVariable("id")]] std::string sku, [[RequestParam("view")]] std::string view, [[RequestHeader("X-Tenant")]] std::string tenant) const;

    [[PostRequest("/products")]]
    Product save([[RequestBody]] const Product& product);

    [[PutRequest("/products/{id}")]]
    void touch([[PathVariable]] int id);

    [[PatchRequest("/products/{id}")]]
    tachyon::json::JsonNode rename([[PathVariable]] int id, [[RequestBody]] const Product& product);

    [[DeleteRequest("/products/{id}")]]
    tachyon::json::JsonNode remove([[PathVariable]] int id);
};
"#,
        ),
        (
            "server/controllers/CatalogController.hpp",
            r#"#pragma once
#include "../remotes/CatalogClient.hpp"

class [[RequestRoute("/catalog")]] [[Public]] CatalogController {
    [[Needs]] CatalogClient& catalog;

public:
    [[GetRoute]]
    tachyon::json::JsonNode getCatalog() {
        tachyon::json::JsonObject answer;
        answer["found"] = catalog.find("anvil", "full", "acme");
        answer["saved"] = catalog.save(Product{"anvil", 3}).level;
        catalog.touch(5);
        answer["renamed"] = catalog.rename(6, Product{"hammer", 1});
        answer["removed"] = catalog.remove(7);
        return answer;
    }
};
"#,
        ),
    ];

    const GATEWAY_FILES: [(&'static str, &'static str); 3] = [
        (
            "server/helpers/report.py",
            r#"import json
import sys

call = json.load(sys.stdin)
mode = call["arguments"][0]
if mode == "fail":
    sys.stderr.write("secret-canary stack trace")
    sys.exit(3)
print(json.dumps({"method": call["method"], "arguments": call["arguments"], "language": "python"}))
"#,
        ),
        (
            "server/remotes/ReportClient.hpp",
            r#"#pragma once

class [[HTTPClient]] ReportClient {
public:
    [[Gateway({"command":["python3","server/helpers/report.py"]})]]
    tachyon::json::JsonNode summarize(tachyon::json::JsonNode mode, const tachyon::json::JsonNode& detail) const;
};
"#,
        ),
        (
            "server/controllers/ReportController.hpp",
            r#"#pragma once

class [[RequestRoute("/report")]] [[Public]] ReportController {
    [[Needs]] ReportClient& reportClient;

public:
    [[PostRoute]]
    tachyon::json::JsonNode postReport([[FromBody]] tachyon::json::JsonNode input) {
        try {
            return reportClient.summarize(input[0], input[1]);
        } catch (const tachyon::TacCodedError& error) {
            tachyon::json::JsonObject failed;
            failed["code"] = error.code();
            failed["message"] = std::string(error.what());
            return failed;
        }
    }
};
"#,
        ),
    ];

    fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        for (relative, source) in files {
            CppProject::write(root.path(), relative, source);
        }
        root
    }

    fn refusal(files: &[(&str, &str)], relative: &str, from: &str, to: &str) -> String {
        let root = Self::project(files);
        let path = root.path().join(relative);
        let source = std::fs::read_to_string(&path).expect("source");
        assert!(source.contains(from), "{relative} holds {from}");
        std::fs::write(&path, source.replace(from, to)).expect("changed source");
        ProjectDiscovery::discover(root.path())
            .expect_err("the C++ project is refused")
            .to_string()
    }

    async fn answers(root: &Path, requests: &[(&str, &str, Option<&str>)]) -> Vec<String> {
        let project = ProjectDiscovery::discover(root).expect("discovery");
        let server = DevServer::bind_project(
            &project,
            &DevServerOptions {
                port: 0,
                watch: false,
                ..DevServerOptions::default()
            },
        )
        .await
        .expect("bind");
        let port = server.address().port();
        let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
        let running = tokio::spawn(server.run_until(async {
            let _ = stopped.await;
        }));
        let mut answers = Vec::new();
        for (method, path, body) in requests {
            let answer = match body {
                Some(body) => Exchange::with_body(port, path, body).await,
                None => Exchange::without_body(port, method, path).await,
            };
            answers.push(answer);
        }
        let _ = stop.send(());
        running.await.expect("server task").expect("server");
        answers
    }
}

#[tokio::test]
async fn a_cpp_http_client_sends_each_request_through_its_setup_adapter() {
    if !CppProject::compiler_available() {
        eprintln!("skipping: clang++ is not installed");
        return;
    }
    let root = CppOutbound::project(&CppOutbound::CLIENT_FILES);
    let answers = CppOutbound::answers(root.path(), &[("GET", "/catalog", None)]).await;
    assert!(answers[0].starts_with("HTTP/1.1 200"), "{}", answers[0]);
    assert_eq!(
        Exchange::body(&answers[0]),
        serde_json::json!({
            "found": {"method": "GET", "uri": "/api/products/anvil?view=full", "tenant": "acme", "body": null, "partition": "catalog"},
            "saved": 3,
            "renamed": {"method": "PATCH", "uri": "/api/products/6", "tenant": "", "body": {"sku": "hammer", "level": 1}, "partition": "catalog"},
            "removed": {"method": "DELETE", "uri": "/api/products/7", "tenant": "", "body": null, "partition": "catalog"},
        })
    );
}

#[test]
fn a_cpp_http_client_needs_its_setup() {
    let files = &CppOutbound::CLIENT_FILES[..];
    let failure = CppOutbound::refusal(
        files,
        "server/setups/CatalogSetup.hpp",
        "class [[Setup]] CatalogSetup",
        "class [[Setup]] OtherSetup",
    );
    assert!(failure.contains("named after"), "{failure}");
    let root = CppOutbound::project(&CppOutbound::CLIENT_FILES[..]);
    std::fs::remove_file(root.path().join("server/setups/CatalogSetup.hpp"))
        .expect("setup removed");
    let failure = ProjectDiscovery::discover(root.path())
        .expect_err("a client without its setup is refused")
        .to_string();
    assert!(failure.contains("TY2015"), "{failure}");
    assert!(
        failure.contains("server/setups/CatalogSetup.hpp"),
        "{failure}"
    );
}

#[test]
fn a_cpp_http_client_method_returns_its_answer() {
    let failure = CppOutbound::refusal(
        &CppOutbound::CLIENT_FILES,
        "server/remotes/CatalogClient.hpp",
        "    Product save(",
        "    Task<Product> save(",
    );
    assert!(failure.contains("Yon C++ runs synchronously"), "{failure}");
}

#[tokio::test]
async fn a_cpp_gateway_calls_a_command_and_sees_only_fixed_failures() {
    if !CppProject::compiler_available() {
        eprintln!("skipping: clang++ is not installed");
        return;
    }
    let root = CppOutbound::project(&CppOutbound::GATEWAY_FILES);
    let answers = CppOutbound::answers(
        root.path(),
        &[
            (
                "POST",
                "/report",
                Some(r#"["echo",{"period":"2026-Q3","totals":[1,2.5,null,true]}]"#),
            ),
            ("POST", "/report", Some(r#"["fail",null]"#)),
        ],
    )
    .await;
    assert!(answers[0].starts_with("HTTP/1.1 200"), "{}", answers[0]);
    assert_eq!(
        Exchange::body(&answers[0]),
        serde_json::json!({
            "method": "summarize",
            "arguments": ["echo", {"period": "2026-Q3", "totals": [1, 2.5, null, true]}],
            "language": "python",
        })
    );
    assert!(!answers[1].contains("secret-canary"), "{}", answers[1]);
    assert_eq!(
        Exchange::body(&answers[1]),
        serde_json::json!({"code": "adapter", "message": "Gateway call failed."})
    );
}

#[test]
fn a_cpp_gateway_is_a_json_signature_in_an_http_client() {
    let files = &CppOutbound::GATEWAY_FILES[..];
    let failure = CppOutbound::refusal(
        files,
        "server/remotes/ReportClient.hpp",
        "const tachyon::json::JsonNode& detail",
        "int detail",
    );
    assert!(failure.contains("TY1312"), "{failure}");
    assert!(
        failure.contains("tachyon::json::JsonNode parameters"),
        "{failure}"
    );
    let failure = CppOutbound::refusal(
        files,
        "server/remotes/ReportClient.hpp",
        "class [[HTTPClient]] ReportClient",
        "class [[Service]] ReportClient",
    );
    assert!(
        failure.contains("goes in an [[HTTPClient]] class"),
        "{failure}"
    );
    let failure = CppOutbound::refusal(
        files,
        "server/remotes/ReportClient.hpp",
        "[[Gateway({\"command\":[\"python3\",\"server/helpers/report.py\"]})]]",
        "[[Gateway({\"command\":[\"python3\"],\"class\":\"x\"})]]",
    );
    assert!(failure.contains("either class"), "{failure}");
}
