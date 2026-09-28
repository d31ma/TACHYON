#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;
use tachyon_core::{DevServer, DevServerOptions, ProjectDiscovery};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

struct IdentityProject {
    extension: &'static str,
    program: &'static str,
    files: &'static [(&'static str, &'static str)],
}

impl IdentityProject {
    const JAVASCRIPT: Self = Self {
        extension: "js",
        program: "node",
        files: &[(
            "server/controllers/AppIdentityProvider.js",
            "@IdentityProvider\nexport class AppIdentityProvider {\n  identify(@FromHeader(\"x-test-user\") user, @FromHeader(\"x-test-roles\") roles) {\n    return { user, roles: roles.split(\",\") };\n  }\n}\n",
        )],
    };

    const TYPESCRIPT: Self = Self {
        extension: "ts",
        program: "node",
        files: &[
            (
                "server/models/Identity.ts",
                "export interface Identity {\n  user: string;\n  roles: string[];\n}\n",
            ),
            (
                "server/controllers/AppIdentityProvider.ts",
                "import type { Identity } from '/models/Identity.ts';\n\n@IdentityProvider\nexport class AppIdentityProvider {\n  identify(@FromHeader(\"x-test-user\") user: string, @FromHeader(\"x-test-roles\") roles: string): Identity {\n    return { user, roles: roles.split(\",\") };\n  }\n}\n",
            ),
        ],
    };

    const PYTHON: Self = Self {
        extension: "py",
        program: "python3",
        files: &[
            (
                "server/models/Identity.py",
                "from dataclasses import dataclass\n\n\n@dataclass\nclass Identity:\n    user: str\n    roles: list[str]\n",
            ),
            (
                "server/controllers/AppIdentityProvider.py",
                "from typing import Annotated\nfrom models import Identity\n\n\n@identity_provider\nclass AppIdentityProvider:\n    def identify(self, user: Annotated[str, from_header(\"x-test-user\")], roles: Annotated[str, from_header(\"x-test-roles\")]) -> Identity:\n        return Identity(user, roles.split(\",\"))\n",
            ),
        ],
    };

    const RUBY: Self = Self {
        extension: "rb",
        program: "ruby",
        files: &[
            (
                "server/models/Identity.rb",
                "# @!attribute [r] user\n#   @return [String]\n# @!attribute [r] roles\n#   @return [Array<String>]\nIdentity = Data.define(:user, :roles)\n",
            ),
            (
                "server/controllers/AppIdentityProvider.rb",
                "#[IdentityProvider]\nclass AppIdentityProvider\n  # @param user [String]\n  # @param roles [String]\n  # @return [Identity]\n  def identify(\n    #[FromHeader(\"x-test-user\")]\n    user,\n    #[FromHeader(\"x-test-roles\")]\n    roles\n  )\n    Identity.new(user: user, roles: roles.split(\",\"))\n  end\nend\n",
            ),
        ],
    };

    const PHP: Self = Self {
        extension: "php",
        program: "php-cgi",
        files: &[
            (
                "server/models/Identity.php",
                "<?php\nclass Identity {\n    /** @param list<string> $roles */\n    public function __construct(public string $user, public array $roles) {}\n}\n",
            ),
            (
                "server/controllers/AppIdentityProvider.php",
                "<?php\nuse Models\\Identity;\n\n#[IdentityProvider]\nclass AppIdentityProvider {\n    public function identify(#[FromHeader(\"x-test-user\")] string $user, #[FromHeader(\"x-test-roles\")] string $roles): Identity {\n        return new Identity($user, explode(',', $roles));\n    }\n}\n",
            ),
        ],
    };

    const JAVA: Self = Self {
        extension: "java",
        program: "java",
        files: &[
            (
                "server/models/Identity.java",
                "record Identity(String user, java.util.List<String> roles) {}\n",
            ),
            (
                "server/controllers/AppIdentityProvider.java",
                "import models.Identity;\n\n@IdentityProvider\nclass AppIdentityProvider {\n    Identity identify(@FromHeader(\"x-test-user\") String user, @FromHeader(\"x-test-roles\") String roles) {\n        return new Identity(user, java.util.List.of(roles.split(\",\")));\n    }\n}\n",
            ),
        ],
    };

    const KOTLIN: Self = Self {
        extension: "kt",
        program: "kotlinc",
        files: &[
            (
                "server/models/Identity.kt",
                "data class Identity(val user: String, val roles: List<String>)\n",
            ),
            (
                "server/controllers/AppIdentityProvider.kt",
                "import models.Identity\n\n@IdentityProvider\nclass AppIdentityProvider {\n    fun identify(@FromHeader(\"x-test-user\") user: String, @FromHeader(\"x-test-roles\") roles: String): Identity {\n        return Identity(user, roles.split(\",\"))\n    }\n}\n",
            ),
        ],
    };

    const CSHARP: Self = Self {
        extension: "cs",
        program: "dotnet",
        files: &[
            (
                "server/models/Identity.cs",
                "using System.Collections.Generic;\n\npublic record Identity(string User, List<string> Roles);\n",
            ),
            (
                "server/controllers/AppIdentityProvider.cs",
                "using System.Collections.Generic;\nusing Models;\n\n[IdentityProvider]\nclass AppIdentityProvider {\n    public Identity Identify([FromHeader(\"x-test-user\")] string user, [FromHeader(\"x-test-roles\")] string roles) {\n        return new Identity(user, new List<string>(roles.Split(',')));\n    }\n}\n",
            ),
        ],
    };

    const RUST: Self = Self {
        extension: "rs",
        program: "rustc",
        files: &[
            (
                "server/models/Identity.rs",
                "pub struct Identity {\n    pub user: String,\n    pub roles: Vec<String>,\n}\n",
            ),
            (
                "server/controllers/AppIdentityProvider.rs",
                "use crate::models::Identity;\n\n#[IdentityProvider]\nstruct AppIdentityProvider {}\nimpl AppIdentityProvider {\n    fn new() -> Self { Self {} }\n\n    fn identify(&self, #[FromHeader(\"x-test-user\")] user: String, #[FromHeader(\"x-test-roles\")] roles: String) -> Identity {\n        Identity { user, roles: roles.split(',').map(String::from).collect() }\n    }\n}\n",
            ),
        ],
    };

    const DART: Self = Self {
        extension: "dart",
        program: "dart",
        files: &[
            (
                "server/models/Identity.dart",
                "class Identity {\n  final String user;\n  final List<String> roles;\n\n  const Identity(this.user, this.roles);\n}\n",
            ),
            (
                "server/controllers/AppIdentityProvider.dart",
                "import 'models/Identity.dart';\n\n@IdentityProvider\nclass AppIdentityProvider {\n  Identity identify(@FromHeader(\"x-test-user\") String user, @FromHeader(\"x-test-roles\") String roles) {\n    return Identity(user, roles.split(','));\n  }\n}\n",
            ),
        ],
    };

    const GO: Self = Self {
        extension: "go",
        program: "go",
        files: &[
            (
                "server/models/Identity.go",
                "type Identity struct {\n\tUser  string   `json:\"user\"`\n\tRoles []string `json:\"roles\"`\n}\n",
            ),
            (
                "server/controllers/AppIdentityProvider.go",
                "import \"strings\"\n\n// @IdentityProvider\ntype AppIdentityProvider struct {\n}\n\nfunc (provider *AppIdentityProvider) Identify(\n// @FromHeader(\"x-test-user\")\n user string, \n// @FromHeader(\"x-test-roles\")\n roles string) Identity {\n\treturn Identity{User: user, Roles: strings.Split(roles, \",\")}\n}\n",
            ),
        ],
    };
}

struct Project;

impl Project {
    const PAGE: &'static str = "client/pages/tac.html";
    const VIEW: &'static str = "client/components/test/view1/tac.html";
    const ROLE_ADMIN: &'static str = "server/roles/Admin.js";
    const ROLE_EDITOR: &'static str = "server/roles/Editor.js";
    const ORDERS: &'static str = "server/controllers/OrdersController.py";
    const ORDERS_SOURCE: &'static str = "from pathlib import Path\n\n\n@request_route(\"/orders\")\n@signed_in\nclass OrdersController:\n    DELETED: str = \"deleted\"\n    @get_route\n    @public\n    def get_orders(self):\n        return {\"list\": \"public\"}\n\n    @post_route\n    def post_order(self):\n        Path(\"MARKERS/posted.marker\").write_text(\"posted\")\n        return {\"created\": True}\n\n    @delete_route\n    @allow(Admin)\n    def delete_orders(self):\n        Path(\"MARKERS/deleted.marker\").write_text(self.DELETED)\n        return {self.DELETED: True}\n";
    const JAVASCRIPT_ORDERS: &'static str = "@RequestRoute(\"/orders\")\n@Allow(Admin)\nexport class OrdersController {\n  @GetRoute\n  getOrders() {\n    return { list: \"admins\" };\n  }\n}\n";

    fn write(root: &Path, relative: &str, source: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("directories");
        fs::write(path, source).expect("source");
    }

    fn base(root: &Path) {
        Self::write(root, Self::PAGE, "<test-view1 />");
        Self::write(
            root,
            Self::VIEW,
            "<main aria-label=\"Home\"><h1>Home</h1></main>",
        );
        Self::write(root, Self::ROLE_ADMIN, "export default class Admin {}\n");
        Self::write(root, Self::ROLE_EDITOR, "export default class Editor {}\n");
    }

    fn guarded_python(root: &Path) {
        Self::base(root);
        let markers = root.to_string_lossy().replace('\\', "/");
        Self::write(
            root,
            Self::ORDERS,
            &Self::ORDERS_SOURCE.replace("MARKERS", &markers),
        );
        for (relative, source) in IdentityProject::JAVASCRIPT.files {
            Self::write(root, relative, source);
        }
    }

    fn guarded_javascript(root: &Path, identity: &IdentityProject) {
        Self::base(root);
        Self::write(
            root,
            "server/controllers/OrdersController.js",
            Self::JAVASCRIPT_ORDERS,
        );
        for (relative, source) in identity.files {
            Self::write(root, relative, source);
        }
    }

    fn refusal(root: &Path) -> String {
        let failure = ProjectDiscovery::discover(root).expect_err("the build refuses the project");
        failure.to_string()
    }
}

struct Server {
    port: u16,
    stop: tokio::sync::oneshot::Sender<()>,
    running: tokio::task::JoinHandle<Result<(), tachyon_core::Failure>>,
}

impl Server {
    async fn start(root: &Path) -> Self {
        let project = ProjectDiscovery::discover(root).expect("project with access rules");
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

    async fn call(&self, method: &str, path: &str, headers: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", self.port))
            .await
            .expect("connect");
        let raw = format!(
            "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n{headers}Content-Length: 0\r\nConnection: close\r\n\r\n"
        );
        stream.write_all(raw.as_bytes()).await.expect("write");
        let mut response = Vec::new();
        tokio::time::timeout(Duration::from_mins(5), stream.read_to_end(&mut response))
            .await
            .expect("answer within the deadline")
            .expect("read");
        String::from_utf8_lossy(&response).into_owned()
    }

    async fn stop(self) {
        self.stop.send(()).expect("stop");
        self.running.await.expect("server task").expect("shutdown");
    }
}

fn available(program: &str) -> bool {
    let flag = match program {
        "kotlinc" => "-version",
        "go" => "version",
        _ => "--version",
    };
    Command::new(program)
        .arg(flag)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[test]
fn a_route_without_an_access_rule_is_refused() {
    let root = tempfile::tempdir().expect("project");
    Project::base(root.path());
    Project::write(
        root.path(),
        "server/controllers/OrdersController.js",
        "@RequestRoute(\"/orders\")\nexport class OrdersController {\n  @GetRoute\n  getOrders() {\n    return [];\n  }\n}\n",
    );
    let refusal = Project::refusal(root.path());
    assert!(refusal.contains("TY1322"), "{refusal}");
    assert!(
        refusal.contains("The route GET /orders in 'server/controllers/OrdersController.js' does not say who may call it"),
        "{refusal}"
    );
}

#[test]
fn a_signed_in_route_without_an_identity_provider_is_refused() {
    let root = tempfile::tempdir().expect("project");
    Project::base(root.path());
    Project::write(root.path(), Project::ORDERS, Project::ORDERS_SOURCE);
    let refusal = Project::refusal(root.path());
    assert!(refusal.contains("TY1323"), "{refusal}");
    assert!(refusal.contains("no @IdentityProvider class"), "{refusal}");
}

#[test]
fn an_unknown_role_type_is_refused() {
    let root = tempfile::tempdir().expect("project");
    Project::guarded_python(root.path());
    fs::remove_file(root.path().join(Project::ROLE_ADMIN)).expect("remove role");
    let refusal = Project::refusal(root.path());
    assert!(
        refusal.contains("names the role Admin, but server/roles has no type named Admin"),
        "{refusal}"
    );
}

#[test]
fn a_second_identity_provider_is_refused() {
    let root = tempfile::tempdir().expect("project");
    Project::guarded_python(root.path());
    Project::write(
        root.path(),
        "server/controllers/OtherIdentityProvider.js",
        "@IdentityProvider\nexport class OtherIdentityProvider {\n  identify(request) {\n    return null;\n  }\n}\n",
    );
    let refusal = Project::refusal(root.path());
    assert!(
        refusal.contains("A project has one identity provider"),
        "{refusal}"
    );
}

#[test]
fn access_markers_are_checked_where_they_are_written() {
    let root = tempfile::tempdir().expect("project");
    Project::base(root.path());
    Project::write(
        root.path(),
        "server/controllers/OrdersController.js",
        "@RequestRoute(\"/orders\")\n@Public\n@SignedIn\nexport class OrdersController {\n  @GetRoute\n  getOrders() {\n    return [];\n  }\n}\n",
    );
    let refusal = Project::refusal(root.path());
    assert!(refusal.contains("cannot be on the same class"), "{refusal}");
}

#[tokio::test]
async fn tachyon_answers_401_and_403_before_the_method_runs() {
    if !available("python3") || !available("node") {
        eprintln!("skipping: python3 or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    Project::guarded_python(root.path());
    let posted = root.path().join("posted.marker");
    let deleted = root.path().join("deleted.marker");
    let server = Server::start(root.path()).await;
    let public = server.call("GET", "/orders", "").await;
    assert!(
        public.contains("200 OK") && public.contains("public"),
        "{public}"
    );
    let anonymous = server.call("POST", "/orders", "").await;
    assert!(anonymous.contains("401 Unauthorized"), "{anonymous}");
    assert!(!posted.exists(), "the method ran without an identity");
    let editor = "x-test-user: ada\r\nx-test-roles: Editor\r\n";
    let refused = server.call("DELETE", "/orders", editor).await;
    assert!(refused.contains("403 Forbidden"), "{refused}");
    assert!(!deleted.exists(), "the method ran for the wrong role");
    let signed_in = server.call("POST", "/orders", editor).await;
    assert!(
        signed_in.contains("200 OK") && signed_in.contains("created"),
        "{signed_in}"
    );
    let admin = "x-test-user: ada\r\nx-test-roles: Editor,Admin\r\n";
    let allowed = server.call("DELETE", "/orders", admin).await;
    assert!(
        allowed.contains("200 OK") && allowed.contains("deleted"),
        "{allowed}"
    );
    server.stop().await;
}

async fn assert_identity_provider(identity: &IdentityProject) {
    if !available(identity.program) || !available("node") {
        eprintln!(
            "skipping {}: {} is unavailable",
            identity.extension, identity.program
        );
        return;
    }
    let root = tempfile::tempdir().expect("project");
    Project::guarded_javascript(root.path(), identity);
    let server = Server::start(root.path()).await;
    let anonymous = server.call("GET", "/orders", "").await;
    assert!(
        anonymous.contains("401 Unauthorized"),
        "{}: {anonymous}",
        identity.extension
    );
    let editor = server
        .call(
            "GET",
            "/orders",
            "x-test-user: ada\r\nx-test-roles: Editor\r\n",
        )
        .await;
    assert!(
        editor.contains("403 Forbidden"),
        "{}: {editor}",
        identity.extension
    );
    let admin = server
        .call(
            "GET",
            "/orders",
            "x-test-user: ada\r\nx-test-roles: Admin\r\n",
        )
        .await;
    assert!(
        admin.contains("200 OK") && admin.contains("admins"),
        "{}: {admin}",
        identity.extension
    );
    server.stop().await;
}

#[tokio::test]
async fn a_javascript_identity_provider_guards_routes() {
    assert_identity_provider(&IdentityProject::JAVASCRIPT).await;
}

#[tokio::test]
async fn a_typescript_identity_provider_guards_routes() {
    assert_identity_provider(&IdentityProject::TYPESCRIPT).await;
}

#[tokio::test]
async fn a_python_identity_provider_guards_routes() {
    assert_identity_provider(&IdentityProject::PYTHON).await;
}

#[tokio::test]
async fn a_ruby_identity_provider_guards_routes() {
    assert_identity_provider(&IdentityProject::RUBY).await;
}

#[tokio::test]
async fn a_php_identity_provider_guards_routes() {
    assert_identity_provider(&IdentityProject::PHP).await;
}

#[tokio::test]
async fn a_java_identity_provider_guards_routes() {
    assert_identity_provider(&IdentityProject::JAVA).await;
}

#[tokio::test]
async fn a_kotlin_identity_provider_guards_routes() {
    assert_identity_provider(&IdentityProject::KOTLIN).await;
}

#[tokio::test]
async fn a_csharp_identity_provider_guards_routes() {
    assert_identity_provider(&IdentityProject::CSHARP).await;
}

#[tokio::test]
async fn a_rust_identity_provider_guards_routes() {
    assert_identity_provider(&IdentityProject::RUST).await;
}

#[tokio::test]
async fn a_dart_identity_provider_guards_routes() {
    assert_identity_provider(&IdentityProject::DART).await;
}

#[tokio::test]
async fn a_go_identity_provider_guards_routes() {
    assert_identity_provider(&IdentityProject::GO).await;
}
