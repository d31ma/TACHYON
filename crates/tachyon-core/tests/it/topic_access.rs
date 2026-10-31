#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;
use std::time::Duration;
use tachyon_core::{DevServer, DevServerOptions, ProjectDiscovery};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

struct TopicLanguage {
    extension: &'static str,
    model: &'static str,
    access: [&'static str; 3],
}

impl TopicLanguage {
    const PYTHON: Self = Self {
        extension: "py",
        model: "from dataclasses import dataclass\n\n\n@topic\nACCESS\n@dataclass\nclass NAME:\n    id: str\n",
        access: ["@public", "@signed_in", "@allow(Admin)"],
    };
    const JAVA: Self = Self {
        extension: "java",
        model: "@Topic\nACCESS\nrecord NAME(String id) {}\n",
        access: ["@Public", "@SignedIn", "@Allow(Admin.class)"],
    };
    const GROOVY: Self = Self {
        extension: "groovy",
        model: "@Topic\nACCESS\nrecord NAME(String id) {}\n",
        access: ["@Public", "@SignedIn", "@Allow(Admin)"],
    };
    const KOTLIN: Self = Self {
        extension: "kt",
        model: "@Topic\nACCESS\ndata class NAME(val id: String)\n",
        access: ["@Public", "@SignedIn", "@Allow(Admin::class)"],
    };
    const DART: Self = Self {
        extension: "dart",
        model: "@Topic\nACCESS\nclass NAME {\n  final String id;\n\n  const NAME(this.id);\n}\n",
        access: ["@Public", "@SignedIn", "@Allow(Admin)"],
    };
    const CSHARP: Self = Self {
        extension: "cs",
        model: "[Topic]\nACCESS\npublic record NAME(string Id);\n",
        access: ["[Public]", "[SignedIn]", "[Allow(typeof(Admin))]"],
    };
    const FSHARP: Self = Self {
        extension: "fs",
        model: "[<Topic>]\nACCESS\ntype NAME = { Id: string }\n",
        access: ["[<Public>]", "[<SignedIn>]", "[<Allow(typeof<Admin>)>]"],
    };
    const RUST: Self = Self {
        extension: "rs",
        model: "#[Topic]\nACCESS\n#[derive(Clone)]\npub struct NAME {\n    pub id: String,\n}\n",
        access: ["#[Public]", "#[SignedIn]", "#[Allow(Admin)]"],
    };
    const GO: Self = Self {
        extension: "go",
        model: "// @Topic\nACCESS\ntype NAME struct {\n\tID string `json:\"id\"`\n}\n",
        access: ["// @Public", "// @SignedIn", "// @Allow(Admin)"],
    };
    const PHP: Self = Self {
        extension: "php",
        model: "<?php\n#[Topic]\nACCESS\nclass NAME {\n    public function __construct(public string $id) {}\n}\n",
        access: ["#[Public]", "#[SignedIn]", "#[Allow(Admin::class)]"],
    };
    const RUBY: Self = Self {
        extension: "rb",
        model: "# @!attribute [r] id\n#   @return [String]\n#[Topic]\nACCESS\nNAME = Data.define(:id)\n",
        access: ["#[Public]", "#[SignedIn]", "#[Allow(Admin)]"],
    };
    const TYPESCRIPT: Self = Self {
        extension: "ts",
        model: "/** @Topic */\nACCESS\nexport interface NAME {\n  id: string\n}\n",
        access: ["/** @Public */", "/** @SignedIn */", "/** @Allow(Admin) */"],
    };
    const JAVASCRIPT: Self = Self {
        extension: "js",
        model: "/**\n * @Topic\nACCESS\n * @typedef {Object} NAME\n * @property {string} id\n */\nexport {}\n",
        access: [" * @Public", " * @SignedIn", " * @Allow(Admin)"],
    };
    const LUA: Self = Self {
        extension: "lua",
        model: "-- @Topic\nACCESS\n---@class NAME\n---@field id string\nNAME = {}\n",
        access: ["-- @Public", "-- @SignedIn", "-- @Allow(Admin)"],
    };
    const CPP: Self = Self {
        extension: "hpp",
        model: "#pragma once\n#include <string>\n\nstruct [[Topic]] ACCESS NAME {\n    std::string id;\n};\n",
        access: ["[[Public]]", "[[SignedIn]]", "[[Allow(Admin)]]"],
    };
    const TOPICS: [&'static str; 3] = ["PriceTick", "ChatLine", "AuditNote"];
    const IDENTITY: &'static str = "@IdentityProvider\nexport class AppIdentityProvider {\n  identify(@FromHeader(\"x-test-user\") user, @FromHeader(\"x-test-roles\") roles) {\n    return { user, roles: roles.split(\",\") };\n  }\n}\n";

    fn write(root: &Path, relative: &str, source: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("directories");
        fs::write(path, source).expect("source");
    }

    fn project(&self, root: &Path) {
        Self::write(root, "client/pages/tac.html", "<test-view1 />");
        Self::write(
            root,
            "client/components/test/view1/tac.html",
            "<main aria-label=\"Home\"><h1>Home</h1></main>",
        );
        Self::write(
            root,
            "server/roles/Admin.js",
            "export default class Admin {}\n",
        );
        Self::write(
            root,
            "server/roles/Editor.js",
            "export default class Editor {}\n",
        );
        Self::write(
            root,
            "server/controllers/AppIdentityProvider.js",
            Self::IDENTITY,
        );
        for (name, access) in Self::TOPICS.iter().zip(self.access) {
            let source = self.model.replace("ACCESS", access).replace("NAME", name);
            Self::write(
                root,
                &format!("server/models/{name}.{}", self.extension),
                &source,
            );
        }
    }

    async fn assert_guarded(&self) {
        if !available("node") {
            eprintln!("skipping {}: node is unavailable", self.extension);
            return;
        }
        let root = tempfile::tempdir().expect("project");
        self.project(root.path());
        let server = Server::start(root.path()).await;
        let editor = "x-test-user: ada\r\nx-test-roles: Editor\r\n";
        let admin = "x-test-user: ada\r\nx-test-roles: Admin\r\n";
        let cases = [
            ("price-tick", "", "200 OK"),
            ("chat-line", "", "401 Unauthorized"),
            ("chat-line", editor, "200 OK"),
            ("audit-note", editor, "403 Forbidden"),
            ("audit-note", admin, "200 OK"),
            ("order-placed", admin, "404 Not Found"),
        ];
        for (topic, headers, status) in cases {
            let head = server.subscribe(topic, headers).await;
            assert!(
                head.contains(status),
                "{} {topic} {headers:?}: {head}",
                self.extension
            );
            let streams = head.contains("text/event-stream");
            assert_eq!(streams, status == "200 OK", "{}: {head}", self.extension);
        }
        server.stop().await;
    }
}

struct Server {
    port: u16,
    stop: tokio::sync::oneshot::Sender<()>,
    running: tokio::task::JoinHandle<Result<(), tachyon_core::Failure>>,
}

impl Server {
    async fn start(root: &Path) -> Self {
        let project = ProjectDiscovery::discover(root).expect("project with topic rules");
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
        let (stop, shutdown) = tokio::sync::oneshot::channel();
        let running = tokio::spawn(server.run_until(async {
            let _ = shutdown.await;
        }));
        Self {
            port,
            stop,
            running,
        }
    }

    async fn subscribe(&self, topic: &str, headers: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", self.port))
            .await
            .expect("connect");
        let raw = format!(
            "GET /.tachyon/topics/{topic} HTTP/1.1\r\nHost: 127.0.0.1\r\nAccept: text/event-stream\r\n{headers}Connection: close\r\n\r\n"
        );
        stream.write_all(raw.as_bytes()).await.expect("write");
        let mut head = Vec::new();
        let mut buffer = [0_u8; 1024];
        while !head.windows(4).any(|window| window == b"\r\n\r\n") {
            let read = tokio::time::timeout(Duration::from_mins(5), stream.read(&mut buffer))
                .await
                .expect("answer within the deadline")
                .expect("read");
            if read == 0 {
                break;
            }
            head.extend_from_slice(&buffer[..read]);
        }
        String::from_utf8_lossy(&head).into_owned()
    }

    async fn stop(self) {
        self.stop.send(()).expect("stop");
        self.running.await.expect("server task").expect("shutdown");
    }
}

fn available(program: &str) -> bool {
    let found = crate::support::program_probe::ProgramProbe::answers(program, "--version");
    crate::support::toolchain::Toolchain::checked(program, found)
}

#[test]
fn a_server_topic_without_an_access_rule_is_refused() {
    let root = tempfile::tempdir().expect("project");
    TopicLanguage::GO.project(root.path());
    TopicLanguage::write(
        root.path(),
        "server/models/PriceTick.go",
        "// @Topic\ntype PriceTick struct {\n\tID string `json:\"id\"`\n}\n",
    );
    let refusal = ProjectDiscovery::discover(root.path())
        .expect_err("the build refuses the project")
        .to_string();
    assert!(
        refusal.contains("TY1322")
            && refusal.contains("The topic PriceTick does not say who may read its stream"),
        "{refusal}"
    );
}

#[test]
fn a_guarded_topic_without_an_identity_provider_is_refused() {
    let root = tempfile::tempdir().expect("project");
    TopicLanguage::PYTHON.project(root.path());
    fs::remove_file(
        root.path()
            .join("server/controllers/AppIdentityProvider.js"),
    )
    .expect("remove identity provider");
    let refusal = ProjectDiscovery::discover(root.path())
        .expect_err("the build refuses the project")
        .to_string();
    assert!(
        refusal.contains("TY1323") && refusal.contains("topic ChatLine"),
        "{refusal}"
    );
}

#[test]
fn a_topic_with_an_unknown_role_or_two_rules_is_refused() {
    let root = tempfile::tempdir().expect("project");
    TopicLanguage::RUST.project(root.path());
    fs::remove_file(root.path().join("server/roles/Admin.js")).expect("remove role");
    TopicLanguage::write(
        root.path(),
        "server/models/ChatLine.go",
        "// @Topic\n// @Public\ntype ChatLine struct {\n\tID string `json:\"id\"`\n}\n",
    );
    let refusal = ProjectDiscovery::discover(root.path())
        .expect_err("the build refuses the project")
        .to_string();
    assert!(
        refusal.contains("names the role Admin, but server/roles has no type named Admin"),
        "{refusal}"
    );
    assert!(
        refusal.contains("The topic ChatLine has one access rule in"),
        "{refusal}"
    );
}

#[tokio::test]
async fn python_topic_streams_follow_their_access_rule() {
    TopicLanguage::PYTHON.assert_guarded().await;
}

#[tokio::test]
async fn java_topic_streams_follow_their_access_rule() {
    TopicLanguage::JAVA.assert_guarded().await;
}

#[tokio::test]
async fn groovy_topic_streams_follow_their_access_rule() {
    TopicLanguage::GROOVY.assert_guarded().await;
}

#[tokio::test]
async fn kotlin_topic_streams_follow_their_access_rule() {
    TopicLanguage::KOTLIN.assert_guarded().await;
}

#[tokio::test]
async fn dart_topic_streams_follow_their_access_rule() {
    TopicLanguage::DART.assert_guarded().await;
}

#[tokio::test]
async fn csharp_topic_streams_follow_their_access_rule() {
    TopicLanguage::CSHARP.assert_guarded().await;
}

#[tokio::test]
async fn fsharp_topic_streams_follow_their_access_rule() {
    TopicLanguage::FSHARP.assert_guarded().await;
}

#[tokio::test]
async fn rust_topic_streams_follow_their_access_rule() {
    TopicLanguage::RUST.assert_guarded().await;
}

#[tokio::test]
async fn go_topic_streams_follow_their_access_rule() {
    TopicLanguage::GO.assert_guarded().await;
}

#[tokio::test]
async fn php_topic_streams_follow_their_access_rule() {
    TopicLanguage::PHP.assert_guarded().await;
}

#[tokio::test]
async fn ruby_topic_streams_follow_their_access_rule() {
    TopicLanguage::RUBY.assert_guarded().await;
}

#[tokio::test]
async fn typescript_topic_streams_follow_their_access_rule() {
    TopicLanguage::TYPESCRIPT.assert_guarded().await;
}

#[tokio::test]
async fn javascript_topic_streams_follow_their_access_rule() {
    TopicLanguage::JAVASCRIPT.assert_guarded().await;
}

#[tokio::test]
async fn lua_topic_streams_follow_their_access_rule() {
    TopicLanguage::LUA.assert_guarded().await;
}

#[tokio::test]
async fn cpp_topic_streams_follow_their_access_rule() {
    TopicLanguage::CPP.assert_guarded().await;
}
