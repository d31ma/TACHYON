#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;
use std::time::Duration;
use tachyon_core::{DevServer, DevServerOptions, ProjectDiscovery};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpStream;

struct ErrorProject;

impl ErrorProject {
    const FILES: [(&'static str, &'static str); 9] = [
        (
            "server/errors/NotFound.hpp",
            "#pragma once\n#include <stdexcept>\n\nclass NotFound : public std::runtime_error {\npublic:\n    using std::runtime_error::runtime_error;\n};\n",
        ),
        (
            "server/errors/OrderNotFound.hpp",
            "#pragma once\n#include \"NotFound.hpp\"\n\nclass OrderNotFound final : public NotFound {\npublic:\n    using NotFound::NotFound;\n};\n",
        ),
        (
            "server/models/Problem.hpp",
            "#pragma once\n#include <string>\n\nstruct Problem {\n    std::string code;\n};\n",
        ),
        (
            "server/models/Identity.hpp",
            "#pragma once\n#include <string>\n#include <vector>\n\nstruct Identity {\n    std::string user;\n    std::vector<std::string> roles;\n};\n",
        ),
        (
            "server/roles/Admin.hpp",
            "#pragma once\n\nclass Admin {};\n",
        ),
        (
            "server/controllers/OrdersErrorHandler.hpp",
            r#"#pragma once
#include "../errors/OrderNotFound.hpp"
#include "../models/Problem.hpp"

class [[ErrorHandler]] OrdersErrorHandler {
public:
    [[HandlesError]] [[ResponseStatus(404)]]
    Problem missing(const OrderNotFound& error) {
        return Problem{"order-not-found"};
    }

    [[HandlesError]] [[ResponseStatus(410)]]
    Problem gone(const NotFound& error, const tachyon::http::HttpRequestMessage& request) {
        return Problem{"gone:" + request.method.toString()};
    }

    [[HandlesError]]
    tachyon::http::HttpResponseMessage refused(const std::invalid_argument& error) {
        tachyon::http::HttpResponseMessage response;
        response.statusCode = tachyon::http::HttpStatusCode::Conflict;
        response.headers.add("X-Reason", "refused");
        response.content = tachyon::http::JsonContent::create(Problem{"conflict"});
        return response;
    }
};
"#,
        ),
        (
            "server/controllers/AppIdentityProvider.hpp",
            r#"#pragma once
#include <optional>
#include <string>
#include "../models/Identity.hpp"

class [[IdentityProvider]] AppIdentityProvider {
public:
    std::optional<Identity> identify(const tachyon::http::HttpRequestMessage& request) {
        std::string token = request.headers.get("Authorization").value_or("");
        if (token == "ada-token") {
            return Identity{"ada", {"Admin"}};
        }
        if (token == "bob-token") {
            return Identity{"bob", {}};
        }
        return std::nullopt;
    }
};
"#,
        ),
        (
            "server/controllers/OrdersController.hpp",
            r#"#pragma once
#include <stdexcept>
#include "../errors/OrderNotFound.hpp"
#include "../models/Problem.hpp"

class [[RequestRoute("/orders")]] [[SignedIn]] OrdersController {
public:
    [[GetRoute("/{id}")]] [[Public]]
    Problem getOrder([[FromPath]] int id) {
        if (id == 1) {
            throw OrderNotFound("order 1");
        }
        if (id == 2) {
            throw NotFound("order 2");
        }
        if (id == 3) {
            throw std::invalid_argument("order 3");
        }
        if (id == 4) {
            throw std::runtime_error("order 4");
        }
        return Problem{"none"};
    }

    [[PostRoute]] [[ResponseStatus(201)]] [[Allow(Admin)]]
    Problem postOrder(const tachyon::http::HttpRequestMessage& request, [[FromQuery("tag")]] std::optional<std::string> tag) {
        std::string body = request.content.has_value() ? request.content->readAsString() : std::string();
        return Problem{request.method.toString() + " " + request.requestUri + " " + body + " " + tag.value_or("")};
    }

    [[PutRoute]]
    tachyon::http::HttpResponseMessage putOrder() {
        tachyon::http::HttpResponseMessage response;
        response.statusCode = tachyon::http::HttpStatusCode::Accepted;
        response.headers.add("Location", "/orders/7");
        response.content = tachyon::http::JsonContent::create(Problem{"accepted"});
        return response;
    }
};
"#,
        ),
        (
            "server/controllers/PlainController.hpp",
            r#"#pragma once
#include "../errors/OrderNotFound.hpp"
#include "../models/Problem.hpp"

class [[RequestRoute("/plain")]] [[Public]] PlainController {
public:
    [[GetRoute]]
    Problem getPlain() {
        throw OrderNotFound("plain");
    }
};
"#,
        ),
    ];

    fn project() -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        for (relative, source) in Self::FILES {
            Self::write(root.path(), relative, source);
        }
        root
    }

    fn write(root: &Path, relative: &str, source: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        fs::write(path, source).expect("source");
    }

    fn compiler_available() -> bool {
        std::process::Command::new("clang++")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
    }

    fn refusal(relative: &str, from: &str, to: &str) -> String {
        let root = Self::project();
        let source = Self::FILES
            .iter()
            .find(|(path, _)| *path == relative)
            .map(|(_, source)| source.replace(from, to))
            .expect("known file");
        assert!(source.contains(to), "{to}");
        Self::write(root.path(), relative, &source);
        ProjectDiscovery::discover(root.path())
            .expect_err("the C++ project is refused")
            .to_string()
    }
}

struct Exchange;

impl Exchange {
    async fn send(port: u16, method: &str, path: &str, headers: &str, body: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", port))
            .await
            .expect("connect");
        let request = format!(
            "{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Length: {}\r\n{headers}\r\n{body}",
            body.len()
        );
        stream.write_all(request.as_bytes()).await.expect("request");
        let mut bytes = Vec::new();
        tokio::time::timeout(Duration::from_mins(2), stream.read_to_end(&mut bytes))
            .await
            .expect("bounded response")
            .expect("response");
        String::from_utf8(bytes).expect("UTF-8 response")
    }

    fn body(response: &str) -> serde_json::Value {
        let (_, body) = response.split_once("\r\n\r\n").expect("response body");
        serde_json::from_str(body).expect("JSON body")
    }
}

#[tokio::test]
async fn cpp_error_handlers_identity_and_native_types_answer_through_clang() {
    if !ErrorProject::compiler_available() {
        eprintln!("skipping: clang++ is not installed");
        return;
    }
    let root = ErrorProject::project();
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

    let nearest = Exchange::send(port, "GET", "/orders/1", "", "").await;
    assert!(nearest.starts_with("HTTP/1.1 404"), "{nearest}");
    assert_eq!(
        Exchange::body(&nearest),
        serde_json::json!({"code": "order-not-found"})
    );
    assert!(!nearest.contains("order 1"), "{nearest}");

    let base = Exchange::send(port, "GET", "/orders/2", "", "").await;
    assert!(base.starts_with("HTTP/1.1 410"), "{base}");
    assert_eq!(
        Exchange::body(&base),
        serde_json::json!({"code": "gone:GET"})
    );

    let native = Exchange::send(port, "GET", "/orders/3", "", "").await;
    assert!(native.starts_with("HTTP/1.1 409"), "{native}");
    assert!(
        native.to_ascii_lowercase().contains("x-reason: refused"),
        "{native}"
    );
    assert_eq!(
        Exchange::body(&native),
        serde_json::json!({"code": "conflict"})
    );

    let unhandled = Exchange::send(port, "GET", "/orders/4", "", "").await;
    assert!(unhandled.starts_with("HTTP/1.1 500"), "{unhandled}");
    assert!(!unhandled.contains("order 4"), "{unhandled}");

    let refused = Exchange::send(port, "GET", "/orders/oops", "", "").await;
    assert!(refused.starts_with("HTTP/1.1 400"), "{refused}");

    let other = Exchange::send(port, "GET", "/plain", "", "").await;
    assert!(other.starts_with("HTTP/1.1 404"), "{other}");

    let json = "Content-Type: application/json\r\n";
    let created = Exchange::send(
        port,
        "POST",
        "/orders?tag=green",
        &format!("{json}Authorization: ada-token\r\n"),
        "{\"item\":\"tea\"}",
    )
    .await;
    assert!(created.starts_with("HTTP/1.1 201"), "{created}");
    assert_eq!(
        Exchange::body(&created),
        serde_json::json!({"code": "POST /orders?tag=green {\"item\":\"tea\"} green"})
    );

    let forbidden = Exchange::send(
        port,
        "POST",
        "/orders",
        &format!("{json}Authorization: bob-token\r\n"),
        "{}",
    )
    .await;
    assert!(forbidden.starts_with("HTTP/1.1 403"), "{forbidden}");

    let anonymous = Exchange::send(port, "POST", "/orders", json, "{}").await;
    assert!(anonymous.starts_with("HTTP/1.1 401"), "{anonymous}");

    let accepted = Exchange::send(port, "PUT", "/orders", "Authorization: bob-token\r\n", "").await;
    assert!(accepted.starts_with("HTTP/1.1 202"), "{accepted}");
    assert!(
        accepted
            .to_ascii_lowercase()
            .contains("location: /orders/7"),
        "{accepted}"
    );
    assert_eq!(
        Exchange::body(&accepted),
        serde_json::json!({"code": "accepted"})
    );

    let _ = stop.send(());
    running.await.expect("server task").expect("server");
}

#[test]
fn cpp_error_handlers_and_error_types_are_checked() {
    let cases = [
        (
            "server/controllers/OrdersErrorHandler.hpp",
            "Problem missing(const OrderNotFound& error)",
            "Problem missing(const std::string& error)",
            "std::string is not a C++ error type",
        ),
        (
            "server/controllers/OrdersErrorHandler.hpp",
            "Problem gone(const NotFound& error,",
            "Problem gone(const OrderNotFound& error,",
            "Two [[HandlesError]] methods handle OrderNotFound",
        ),
        (
            "server/controllers/OrdersErrorHandler.hpp",
            "[[ResponseStatus(404)]]",
            "[[ResponseStatus(204)]]",
            "from 400 to 599",
        ),
        (
            "server/controllers/OrdersErrorHandler.hpp",
            "Problem missing(const OrderNotFound& error)",
            "Problem missing(const OrderNotFound& error, int extra)",
            "optionally, the request",
        ),
        (
            "server/errors/NotFound.hpp",
            "class NotFound : public std::runtime_error {",
            "class NotFound {",
            "derives from std::exception",
        ),
        (
            "server/errors/NotFound.hpp",
            "class NotFound : public std::runtime_error {",
            "class NotFound : public Missing {",
            "derives from std::exception",
        ),
        (
            "server/errors/NotFound.hpp",
            "class NotFound :",
            "class [[Service]] NotFound :",
            "without Tachyon markers",
        ),
        (
            "server/controllers/AppIdentityProvider.hpp",
            "class [[IdentityProvider]] AppIdentityProvider {",
            "class [[IdentityProvider]] [[SignedIn]] AppIdentityProvider {",
            "[[SignedIn]] belongs in a class marked [[RequestRoute]]",
        ),
    ];
    for (path, from, to, expected) in cases {
        let failure = ErrorProject::refusal(path, from, to);
        assert!(failure.contains(expected), "{to}: {failure}");
    }
}

#[test]
fn a_cpp_error_handler_without_handler_methods_is_refused() {
    let root = ErrorProject::project();
    ErrorProject::write(
        root.path(),
        "server/controllers/EmptyErrorHandler.hpp",
        "#pragma once\n\nclass [[ErrorHandler]] EmptyErrorHandler {\npublic:\n    int count() { return 1; }\n};\n",
    );
    let failure = ProjectDiscovery::discover(root.path())
        .expect_err("the empty handler is refused")
        .to_string();
    assert!(
        failure.contains("needs at least one [[HandlesError]]"),
        "{failure}"
    );
}
