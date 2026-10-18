#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;
use std::time::Duration;
use tachyon_contracts::{HandlerRequest, HttpMethod};
use tachyon_core::{
    DevServer, DevServerOptions, HandlerCancellation, HandlerSource, HandlerSupervisor,
    HandlerSupervisorOptions, ProjectDiscovery,
};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpStream;

struct CppProject;

impl CppProject {
    const MODEL: &'static str = r"#pragma once
#include <optional>
#include <string>

struct Order {
    std::string item;
    int count = 1;
    std::optional<std::string> note;
};
";
    const CONTROLLER: &'static str = r#"#pragma once
#include <stdexcept>
#include <string>
#include <vector>
#include "../models/Order.hpp"

class [[RequestRoute("/orders")]] [[Public]] OrderController {
public:
    [[GetRoute("/{id}")]]
    Order getOrder([[FromPath]] int id, [[FromQuery("view")]] std::optional<std::string> view) {
        Order order;
        order.item = "tea";
        order.count = id;
        order.note = view;
        return order;
    }

    [[PostRoute]]
    std::vector<Order> postOrder([[FromBody]] const Order& order) {
        if (order.count > 9) {
            throw std::runtime_error("too many cups");
        }
        return {order, order};
    }

    [[DeleteRoute("/{id}")]]
    void deleteOrder([[FromPath]] int id) {
        removed += id;
    }

private:
    int removed = 0;
};
"#;

    fn write(root: &Path, relative: &str, source: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        fs::write(path, source).expect("source");
    }

    fn with_controller(controller: &str) -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        Self::write(root.path(), "server/models/Order.hpp", Self::MODEL);
        Self::write(
            root.path(),
            "server/controllers/OrderController.hpp",
            controller,
        );
        root
    }

    fn compiler_available() -> bool {
        std::process::Command::new("clang++")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
    }

    fn refusal(controller: &str) -> String {
        let root = Self::with_controller(controller);
        ProjectDiscovery::discover(root.path())
            .expect_err("the C++ controller is refused")
            .to_string()
    }
}

struct Exchange;

impl Exchange {
    async fn send(port: u16, request: String) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", port))
            .await
            .expect("connect");
        stream.write_all(request.as_bytes()).await.expect("request");
        let mut bytes = Vec::new();
        tokio::time::timeout(Duration::from_mins(2), stream.read_to_end(&mut bytes))
            .await
            .expect("bounded response")
            .expect("response");
        String::from_utf8(bytes).expect("UTF-8 response")
    }

    async fn without_body(port: u16, method: &str, path: &str) -> String {
        Self::send(
            port,
            format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"),
        )
        .await
    }

    async fn with_body(port: u16, path: &str, body: &str) -> String {
        Self::send(
            port,
            format!(
                "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            ),
        )
        .await
    }

    fn body(response: &str) -> serde_json::Value {
        let (_, body) = response.split_once("\r\n\r\n").expect("response body");
        serde_json::from_str(body).expect("JSON body")
    }
}

#[tokio::test]
async fn a_cpp_controller_answers_typed_routes_through_clang() {
    if !CppProject::compiler_available() {
        eprintln!("skipping: clang++ is not installed");
        return;
    }
    let root = CppProject::with_controller(CppProject::CONTROLLER);
    let project = ProjectDiscovery::discover(root.path()).expect("discovery");
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

    let found = Exchange::without_body(port, "GET", "/orders/7?view=full").await;
    assert!(found.starts_with("HTTP/1.1 200"), "{found}");
    assert_eq!(
        Exchange::body(&found),
        serde_json::json!({"item": "tea", "count": 7, "note": "full"})
    );

    let created = Exchange::with_body(port, "/orders", r#"{"item":"green","count":2}"#).await;
    assert!(created.starts_with("HTTP/1.1 200"), "{created}");
    assert_eq!(
        Exchange::body(&created),
        serde_json::json!([
            {"item": "green", "count": 2, "note": null},
            {"item": "green", "count": 2, "note": null}
        ])
    );

    let refused = Exchange::with_body(port, "/orders", r#"{"count":2}"#).await;
    assert!(refused.starts_with("HTTP/1.1 400"), "{refused}");

    let failed = Exchange::with_body(port, "/orders", r#"{"item":"green","count":20}"#).await;
    assert!(failed.starts_with("HTTP/1.1 500"), "{failed}");
    assert!(!failed.contains("too many cups"), "{failed}");

    let removed = Exchange::without_body(port, "DELETE", "/orders/3").await;
    assert!(removed.starts_with("HTTP/1.1 204"), "{removed}");

    let _ = stop.send(());
    running.await.expect("server task").expect("server");
}

#[test]
fn a_cpp_marker_that_yon_cpp_lacks_is_refused_by_name() {
    let controller = CppProject::CONTROLLER.replace(
        "[[GetRoute(\"/{id}\")]]",
        "[[GetRoute(\"/{id}\")]] [[Cached]]",
    );
    let failure = CppProject::refusal(&controller);
    assert!(failure.contains("TY1308"), "{failure}");
    assert!(failure.contains("[[Cached]]"), "{failure}");
    assert!(failure.contains("C++ gets it in a later step"), "{failure}");
}

#[test]
fn a_cpp_route_parameter_without_a_source_marker_is_refused() {
    let controller = CppProject::CONTROLLER.replace("[[FromPath]] int id)", "int id)");
    let failure = CppProject::refusal(&controller);
    assert!(failure.contains("TY2015"), "{failure}");
    assert!(failure.contains("[[FromPath]], [[FromQuery]]"), "{failure}");
}

#[test]
fn a_cpp_socket_controller_is_refused_until_cpp_sockets_land() {
    let controller = CppProject::CONTROLLER.replace(
        "class [[RequestRoute(\"/orders\")]] [[Public]] OrderController",
        "class [[SocketRoute(\"/orders\")]] [[Public]] OrderController",
    );
    let failure = CppProject::refusal(&controller);
    assert!(failure.contains("[[SocketRoute]]"), "{failure}");
    assert!(failure.contains("C++ gets it in a later step"), "{failure}");
}

struct CppRoles;

impl CppRoles {
    const FILES: [(&'static str, &'static str); 5] = [
        (
            "server/models/PlacedOrder.hpp",
            "#pragma once\n#include <string>\n\nstruct PlacedOrder {\n    std::string status;\n    int total = 0;\n};\n",
        ),
        (
            "server/models/Ledger.hpp",
            "#pragma once\n#include <vector>\n#include <string>\n\nstruct Ledger {\n    int total = 0;\n    std::vector<std::string> log;\n};\n",
        ),
        (
            "server/services/LedgerService.hpp",
            r#"#pragma once
#include <string>
#include <vector>

class [[Service]] LedgerService {
    ApplicationEventPublisher applicationEventPublisher;
    [[Needs]] LedgerAmountsHelper& amounts;

public:
    std::vector<std::string> log;

    int subtotal() const { return amounts.subtotal(); }

private:
    [[OnStart]]
    void open() { applicationEventPublisher.publishEvent(PlacedOrder{"paid", 40}); }

    [[OnEvent({"condition":"value.total >= 10"})]]
    void placed(const PlacedOrder& order) { log.push_back(order.status); }
};
"#,
        ),
        (
            "server/helpers/LedgerAmountsHelper.hpp",
            "#pragma once\n\nclass [[Helper, Lifetime(\"singleton\")]] LedgerAmountsHelper {\npublic:\n    int subtotal() const { return 40; }\n    int surcharge() const { return 2; }\n};\n",
        ),
        (
            "server/controllers/LedgerController.hpp",
            r#"#pragma once
#include "../services/LedgerService.hpp"

class [[RequestRoute("/ledger")]] [[Public]] LedgerController {
    [[Needs]] [[Named("LedgerService")]] LedgerService& service;
    [[Needs]] LedgerAmountsHelper& amounts;

public:
    [[GetRoute]]
    Ledger getLedger() { return Ledger{service.subtotal() + amounts.surcharge(), service.log}; }
};
"#,
        ),
    ];

    fn project() -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        for (relative, source) in Self::FILES {
            CppProject::write(root.path(), relative, source);
        }
        root
    }
}

#[tokio::test]
async fn a_cpp_controller_reaches_its_services_helpers_and_events() {
    if !CppProject::compiler_available() {
        eprintln!("skipping: clang++ is not installed");
        return;
    }
    let root = CppRoles::project();
    let project = ProjectDiscovery::discover(root.path()).expect("discovery");
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

    let ledger = Exchange::without_body(port, "GET", "/ledger").await;
    assert!(ledger.starts_with("HTTP/1.1 200"), "{ledger}");
    assert_eq!(
        Exchange::body(&ledger),
        serde_json::json!({"total": 42, "log": ["paid"]})
    );

    let _ = stop.send(());
    running.await.expect("server task").expect("server");
}

#[tokio::test]
async fn a_scheduled_cpp_worker_runs_with_its_helpers() {
    if !CppProject::compiler_available() {
        eprintln!("skipping: clang++ is not installed");
        return;
    }
    let root = CppRoles::project();
    let relative = "server/workers/beat.hpp";
    CppProject::write(
        root.path(),
        relative,
        "#pragma once\n\nclass [[Schedule({\"fixedDelaySeconds\":30})]] Handler {\n    [[Needs]] LedgerAmountsHelper& amounts;\n\npublic:\n    int POST() { return amounts.subtotal(); }\n};\n",
    );
    ProjectDiscovery::discover(root.path()).expect("discovery");
    let source = HandlerSource::discover(root.path(), relative).expect("worker discovery");
    let supervisor =
        HandlerSupervisor::new(HandlerSupervisorOptions::default()).expect("supervisor");
    let response = supervisor
        .invoke(
            &source,
            &HandlerRequest::route("cpp_worker", "/", HttpMethod::Post),
            &HandlerCancellation::default(),
        )
        .await
        .expect("invocation");
    assert!(response.error.is_none(), "{:?}", response.error);
    assert_eq!(response.body.expect("body").data, "40");
}

#[test]
fn a_cpp_needs_field_names_a_class_in_the_role_folders() {
    let root = CppRoles::project();
    std::fs::remove_file(root.path().join("server/helpers/LedgerAmountsHelper.hpp"))
        .expect("helper removed");
    let failure = ProjectDiscovery::discover(root.path())
        .expect_err("a missing provider is refused")
        .to_string();
    assert!(failure.contains("TY2015"), "{failure}");
    assert!(
        failure.contains("there is no LedgerAmountsHelper.hpp"),
        "{failure}"
    );
}

#[test]
fn a_cpp_compiler_error_names_the_authored_file() {
    if !CppProject::compiler_available() {
        eprintln!("skipping: clang++ is not installed");
        return;
    }
    let controller = CppProject::CONTROLLER.replace("removed += id;", "removed += missingName;");
    let failure = CppProject::refusal(&controller);
    assert!(failure.contains("failed to compile"), "{failure}");
    assert!(
        failure.contains("server/controllers/OrderController.hpp"),
        "{failure}"
    );
    assert!(failure.contains("missingName"), "{failure}");
}

struct CppContext;

impl CppContext {
    const FILES: [(&'static str, &'static str); 6] = [
        (
            "server/application.json",
            r#"{"active":{"profile":"production"},"catalog":{"page":{"size":7},"cache":{"enabled":true}}}"#,
        ),
        (
            "server/models/CatalogSettings.hpp",
            "#pragma once\n\nstruct [[Settings({\"prefix\":\"YON_CATALOG_\"})]] CatalogSettings {\n    int pageSize = 20;\n    bool cacheEnabled = false;\n};\n",
        ),
        (
            "server/models/Endpoint.hpp",
            "#pragma once\n#include <string>\n\nstruct Endpoint {\n    [[Setting(\"${yon.active.profile:development}\")]] std::string profile;\n};\n",
        ),
        (
            "server/models/Catalog.hpp",
            "#pragma once\n#include <string>\n\nstruct Catalog {\n    std::string profile;\n    int pageSize = 0;\n    std::string environment;\n};\n",
        ),
        (
            "server/setups/CatalogSetup.hpp",
            r#"#pragma once

class [[Setup, WhenProfile("production"), WhenSetting({"name":"YON_CATALOG_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})]] CatalogSetup {
public:
    static BehaviorAdapters configure(const BehaviorExecution& execution) {
        BehaviorAdapters adapters;
        adapters.http["environment"] = [environment = execution.environment](const tachyon::http::HttpRequestMessage&) {
            return tachyon::json::JsonNode(environment);
        };
        return adapters;
    }
};
"#,
        ),
        (
            "server/controllers/CatalogController.hpp",
            r#"#pragma once
#include "../models/Catalog.hpp"

class [[RequestRoute("/catalog")]] [[Public]] CatalogController {
public:
    [[GetRoute]]
    Catalog getCatalog() {
        tachyon::json::JsonNode application = tachyon::yon::YonApplication::snapshot();
        Endpoint endpoint = Behaviors::values<Endpoint>(application);
        CatalogSettings settings = Behaviors::properties<CatalogSettings>(application);
        BehaviorOptions options;
        options.partition = "public-catalog";
        options.profiles = {endpoint.profile};
        BehaviorContext context = Behaviors::create<CatalogSetup>(options.withApplication(application));
        tachyon::http::HttpRequestMessage request(tachyon::http::HttpMethod::Get, "/environment");
        std::string environment = context.adapters().http.at("environment")(request).getValue<std::string>();
        context.dispose();
        return Catalog{endpoint.profile, settings.pageSize, environment};
    }

    [[GetRoute("/development")]]
    Catalog getDevelopment() {
        BehaviorOptions options;
        options.partition = "public-catalog";
        options.profiles = {"development"};
        Behaviors::create<CatalogSetup>(options.withApplication(tachyon::yon::YonApplication::snapshot()));
        return Catalog{};
    }
};
"#,
        ),
    ];

    fn project() -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        for (relative, source) in Self::FILES {
            CppProject::write(root.path(), relative, source);
        }
        root
    }

    fn refusal(relative: &str, from: &str, to: &str) -> String {
        let root = Self::project();
        let path = root.path().join(relative);
        let source = fs::read_to_string(&path).expect("source");
        assert!(source.contains(from), "{relative} holds {from}");
        fs::write(&path, source.replace(from, to)).expect("changed source");
        ProjectDiscovery::discover(root.path())
            .expect_err("the C++ project is refused")
            .to_string()
    }
}

#[tokio::test]
async fn a_cpp_controller_binds_settings_and_opens_a_setup_context() {
    if !CppProject::compiler_available() {
        eprintln!("skipping: clang++ is not installed");
        return;
    }
    let root = CppContext::project();
    let project = ProjectDiscovery::discover(root.path()).expect("discovery");
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

    let catalog = Exchange::without_body(port, "GET", "/catalog").await;
    assert!(catalog.starts_with("HTTP/1.1 200"), "{catalog}");
    assert_eq!(
        Exchange::body(&catalog),
        serde_json::json!({"profile": "production", "pageSize": 7, "environment": "yon"})
    );

    let inactive = Exchange::without_body(port, "GET", "/catalog/development").await;
    assert!(inactive.starts_with("HTTP/1.1 500"), "{inactive}");

    let _ = stop.send(());
    running.await.expect("server task").expect("server");
}

#[test]
fn a_cpp_setup_names_yon_settings() {
    let failure = CppContext::refusal(
        "server/setups/CatalogSetup.hpp",
        "YON_CATALOG_CACHE_ENABLED",
        "TAC_CATALOG_CACHE_ENABLED",
    );
    assert!(failure.contains("must use YON_"), "{failure}");
    let failure = CppContext::refusal(
        "server/models/CatalogSettings.hpp",
        "YON_CATALOG_",
        "TAC_CATALOG_",
    );
    assert!(failure.contains("must use YON_"), "{failure}");
    let failure = CppContext::refusal(
        "server/models/Endpoint.hpp",
        "${yon.active.profile",
        "${tac.active.profile",
    );
    assert!(failure.contains("yon namespace"), "{failure}");
}

#[test]
fn a_cpp_setup_lives_in_server_setups() {
    let failure = CppContext::refusal(
        "server/controllers/CatalogController.hpp",
        "[[RequestRoute(\"/catalog\")]] [[Public]]",
        "[[RequestRoute(\"/catalog\")]] [[Public]] [[Setup]]",
    );
    assert!(failure.contains("server/setups/<Name>.hpp"), "{failure}");
    let failure = CppContext::refusal(
        "server/setups/CatalogSetup.hpp",
        "static BehaviorAdapters configure(",
        "static BehaviorAdapters build(",
    );
    assert!(
        failure.contains("static BehaviorAdapters configure"),
        "{failure}"
    );
}
