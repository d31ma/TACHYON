#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;
use tachyon_contracts::{HandlerRequest, HttpMethod};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
};

struct OutboundVerbs {
    folder: &'static str,
    extension: &'static str,
    program: &'static str,
    probe: &'static str,
}

impl OutboundVerbs {
    const JAVASCRIPT: Self = Self {
        folder: "js",
        extension: "js",
        program: "node",
        probe: "--version",
    };
    const TYPESCRIPT: Self = Self {
        folder: "ts",
        extension: "ts",
        program: "node",
        probe: "--version",
    };
    const PYTHON: Self = Self {
        folder: "py",
        extension: "py",
        program: "python3",
        probe: "--version",
    };
    const PHP: Self = Self {
        folder: "php",
        extension: "php",
        program: "php-cgi",
        probe: "--version",
    };
    const JAVA: Self = Self {
        folder: "java",
        extension: "java",
        program: "javac",
        probe: "-version",
    };
    const GROOVY: Self = Self {
        folder: "groovy",
        extension: "groovy",
        program: "groovyc",
        probe: "--version",
    };
    const KOTLIN: Self = Self {
        folder: "kt",
        extension: "kt",
        program: "kotlinc",
        probe: "-version",
    };
    const CSHARP: Self = Self {
        folder: "cs",
        extension: "cs",
        program: "dotnet",
        probe: "--version",
    };
    const FSHARP: Self = Self {
        folder: "fs",
        extension: "fs",
        program: "dotnet",
        probe: "--version",
    };
    const RUST: Self = Self {
        folder: "rs",
        extension: "rs",
        program: "cargo",
        probe: "--version",
    };
    const DART: Self = Self {
        folder: "dart",
        extension: "dart",
        program: "dart",
        probe: "--version",
    };
    const GO: Self = Self {
        folder: "go",
        extension: "go",
        program: "go",
        probe: "version",
    };
    const RUBY: Self = Self {
        folder: "rb",
        extension: "rb",
        program: "ruby",
        probe: "--version",
    };
    const LUA: Self = Self {
        folder: "lua",
        extension: "lua",
        program: "lua",
        probe: "-v",
    };

    fn fixture(&self) -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/outbound_verbs")
            .join(self.folder)
    }

    fn available(&self) -> bool {
        let found = crate::support::program_probe::ProgramProbe::answers(self.program, self.probe);
        crate::support::toolchain::Toolchain::checked(self.program, found)
    }

    fn copy(from: &Path, to: &Path) {
        fs::create_dir_all(to).expect("project folder");
        for entry in fs::read_dir(from).expect("fixture folder") {
            let entry = entry.expect("fixture entry");
            let target = to.join(entry.file_name());
            if entry.file_type().expect("entry type").is_dir() {
                Self::copy(&entry.path(), &target);
            } else {
                fs::copy(entry.path(), target).expect("fixture file");
            }
        }
    }

    async fn answer(&self, root: &Path) -> serde_json::Value {
        let entry = format!("server/controllers/CatalogController.{}", self.extension);
        let source = HandlerSource::discover(root, &entry)
            .unwrap_or_else(|failure| panic!("{}: {failure}", self.folder));
        let supervisor =
            HandlerSupervisor::new(HandlerSupervisorOptions::default()).expect("supervisor");
        let request = HandlerRequest::route("outbound-verbs", "/catalog", HttpMethod::Get);
        let response = supervisor
            .invoke(&source, &request, &HandlerCancellation::default())
            .await
            .expect("invocation");
        assert!(
            response.error.is_none(),
            "{}: {:?}",
            self.folder,
            response.error
        );
        let body = response.body.expect("body").data;
        serde_json::from_str(&body).unwrap_or_else(|_| panic!("{}: {body}", self.folder))
    }

    fn header<'a>(sent: &'a serde_json::Value, name: &str) -> Option<&'a serde_json::Value> {
        sent["headers"]
            .as_object()?
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value)
    }

    fn assert_sent(&self, sent: &serde_json::Value) {
        let folder = self.folder;
        let expected = [
            (
                "found",
                "GET",
                "/api/products/anvil",
                serde_json::Value::Null,
            ),
            (
                "saved",
                "POST",
                "/api/products",
                serde_json::json!({"sku": "anvil", "level": 3}),
            ),
            (
                "replaced",
                "PUT",
                "/api/products/5",
                serde_json::json!({"sku": "anvil", "level": 4}),
            ),
            (
                "renamed",
                "PATCH",
                "/api/products/6",
                serde_json::json!({"sku": "hammer", "level": 1}),
            ),
            (
                "removed",
                "DELETE",
                "/api/products/7",
                serde_json::Value::Null,
            ),
        ];
        for (call, method, path, request_body) in expected {
            let request = &sent[call];
            assert_eq!(request["method"], method, "{folder} {call}: {sent}");
            assert_eq!(request["path"], path, "{folder} {call}: {sent}");
            assert_eq!(request["body"], request_body, "{folder} {call}: {sent}");
        }
        assert_eq!(sent["found"]["query"]["view"], "full", "{folder}: {sent}");
        assert_eq!(
            Self::header(&sent["found"], "X-Tenant"),
            Some(&serde_json::json!("acme")),
            "{folder}: {sent}"
        );
        assert_eq!(
            Self::header(&sent["saved"], "X-Tenant"),
            None,
            "{folder}: {sent}"
        );
    }

    async fn run(self) {
        if !self.available() {
            eprintln!("skipping {}: {} is unavailable", self.folder, self.program);
            return;
        }
        let root = tempfile::tempdir().expect("project");
        Self::copy(&self.fixture(), root.path());
        let sent = self.answer(root.path()).await;
        self.assert_sent(&sent);
    }
}

#[tokio::test]
async fn a_javascript_http_client_sends_every_request_marker() {
    OutboundVerbs::JAVASCRIPT.run().await;
}

#[tokio::test]
async fn a_typescript_http_client_sends_every_request_marker() {
    OutboundVerbs::TYPESCRIPT.run().await;
}

#[tokio::test]
async fn a_python_http_client_sends_every_request_marker() {
    OutboundVerbs::PYTHON.run().await;
}

#[tokio::test]
async fn a_php_http_client_sends_every_request_marker() {
    OutboundVerbs::PHP.run().await;
}

#[tokio::test]
async fn a_java_http_client_sends_every_request_marker() {
    OutboundVerbs::JAVA.run().await;
}

#[tokio::test]
async fn a_groovy_http_client_sends_every_request_marker() {
    OutboundVerbs::GROOVY.run().await;
}

#[tokio::test]
async fn a_kotlin_http_client_sends_every_request_marker() {
    OutboundVerbs::KOTLIN.run().await;
}

#[tokio::test]
async fn a_csharp_http_client_sends_every_request_marker() {
    OutboundVerbs::CSHARP.run().await;
}

#[tokio::test]
async fn a_fsharp_http_client_sends_every_request_marker() {
    OutboundVerbs::FSHARP.run().await;
}

#[tokio::test]
async fn a_rust_http_client_sends_every_request_marker() {
    OutboundVerbs::RUST.run().await;
}

#[tokio::test]
async fn a_dart_http_client_sends_every_request_marker() {
    OutboundVerbs::DART.run().await;
}

#[tokio::test]
async fn a_go_http_client_sends_every_request_marker() {
    OutboundVerbs::GO.run().await;
}

#[tokio::test]
async fn a_ruby_http_client_sends_every_request_marker() {
    OutboundVerbs::RUBY.run().await;
}

#[tokio::test]
async fn a_lua_http_client_sends_every_request_marker() {
    OutboundVerbs::LUA.run().await;
}
