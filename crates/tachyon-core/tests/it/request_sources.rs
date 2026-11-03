#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;
use std::time::Duration;
use tachyon_core::{DevServer, DevServerOptions, ProjectDiscovery};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpStream;

struct SourcesProject {
    extension: &'static str,
    program: &'static str,
    version_flag: &'static str,
    files: &'static [(&'static str, &'static str)],
}

impl SourcesProject {
    const JAVASCRIPT: Self = Self {
        extension: "js",
        program: "node",
        version_flag: "--version",
        files: &[
            (
                "server/models/OrderView.js",
                "/**\n * @typedef {Object} OrderView\n * @property {number} id\n * @property {string} tenant\n * @property {string} theme\n */\nexport {}\n",
            ),
            (
                "server/controllers/OrdersController.js",
                "@RequestRoute(\"/orders\")\n@Public\nexport class OrdersController {\n  /**\n   * @param {number} id\n   * @param {string} tenant\n   * @param {string} theme\n   * @returns {OrderView}\n   */\n  @PutRoute(\"/{id}\")\n  putOrder(@FromPath id, @FromHeader(\"x-tenant\") tenant, @FromCookie(\"theme\") theme) {\n    return { id, tenant, theme };\n  }\n\n  /**\n   * @param {number} id\n   * @param {string} theme\n   * @returns {OrderView}\n   */\n  @PatchRoute(\"/{id}\")\n  patchOrder(@FromPath id, @FromCookie(\"theme\") theme) {\n    return { id, tenant: \"patched\", theme };\n  }\n}\n",
            ),
        ],
    };

    const TYPESCRIPT: Self = Self {
        extension: "ts",
        program: "node",
        version_flag: "--version",
        files: &[
            (
                "server/models/OrderView.ts",
                "export interface OrderView {\n  id: number;\n  tenant: string;\n  theme: string;\n}\n",
            ),
            (
                "server/controllers/OrdersController.ts",
                "import type { OrderView } from '/models/OrderView.ts';\n\n@RequestRoute(\"/orders\")\n@Public\nexport class OrdersController {\n  @PutRoute(\"/{id}\")\n  putOrder(@FromPath id: number, @FromHeader(\"x-tenant\") tenant: string, @FromCookie(\"theme\") theme: string): OrderView {\n    return { id, tenant, theme };\n  }\n\n  @PatchRoute(\"/{id}\")\n  patchOrder(@FromPath id: number, @FromCookie(\"theme\") theme: string): OrderView {\n    return { id, tenant: \"patched\", theme };\n  }\n}\n",
            ),
        ],
    };

    const PYTHON: Self = Self {
        extension: "py",
        program: "python3",
        version_flag: "--version",
        files: &[
            (
                "server/models/OrderView.py",
                "from dataclasses import dataclass\n\n\n@dataclass\nclass OrderView:\n    id: int\n    tenant: str\n    theme: str\n",
            ),
            (
                "server/controllers/OrdersController.py",
                "from typing import Annotated\nfrom models import OrderView\n\n\n@request_route(\"/orders\")\n@public\nclass OrdersController:\n    @put_route(\"/{id}\")\n    def put_order(self, id: Annotated[int, from_path], tenant: Annotated[str, from_header(\"x-tenant\")], theme: Annotated[str, from_cookie(\"theme\")]) -> OrderView:\n        return OrderView(id, tenant, theme)\n\n    @patch_route(\"/{id}\")\n    def patch_order(self, id: Annotated[int, from_path], theme: Annotated[str, from_cookie(\"theme\")]) -> OrderView:\n        return OrderView(id, \"patched\", theme)\n",
            ),
        ],
    };

    const RUBY: Self = Self {
        extension: "rb",
        program: "ruby",
        version_flag: "--version",
        files: &[
            (
                "server/models/OrderView.rb",
                "# @!attribute [r] id\n#   @return [Integer]\n# @!attribute [r] tenant\n#   @return [String]\n# @!attribute [r] theme\n#   @return [String]\nOrderView = Data.define(:id, :tenant, :theme)\n",
            ),
            (
                "server/controllers/OrdersController.rb",
                "#[RequestRoute(\"/orders\")]\n#[Public]\nclass OrdersController\n  # @param id [Integer]\n  # @param tenant [String]\n  # @param theme [String]\n  # @return [OrderView]\n  #[PutRoute(\"/{id}\")]\n  def put_order(\n    #[FromPath]\n    id,\n    #[FromHeader(\"x-tenant\")]\n    tenant,\n    #[FromCookie(\"theme\")]\n    theme\n  )\n    OrderView.new(id: id, tenant: tenant, theme: theme)\n  end\n\n  # @param id [Integer]\n  # @param theme [String]\n  # @return [OrderView]\n  #[PatchRoute(\"/{id}\")]\n  def patch_order(\n    #[FromPath]\n    id,\n    #[FromCookie(\"theme\")]\n    theme\n  )\n    OrderView.new(id: id, tenant: \"patched\", theme: theme)\n  end\nend\n",
            ),
        ],
    };

    const PHP: Self = Self {
        extension: "php",
        program: "php-cgi",
        version_flag: "--version",
        files: &[
            (
                "server/models/OrderView.php",
                "<?php\nclass OrderView {\n    public function __construct(public int $id, public string $tenant, public string $theme) {}\n}\n",
            ),
            (
                "server/controllers/OrdersController.php",
                "<?php\nuse Models\\OrderView;\n\n#[RequestRoute(\"/orders\")]\n#[Public]\nclass OrdersController {\n    #[PutRoute(\"/{id}\")]\n    public function putOrder(#[FromPath] int $id, #[FromHeader(\"x-tenant\")] string $tenant, #[FromCookie(\"theme\")] string $theme): OrderView {\n        return new OrderView($id, $tenant, $theme);\n    }\n\n    #[PatchRoute(\"/{id}\")]\n    public function patchOrder(#[FromPath] int $id, #[FromCookie(\"theme\")] string $theme): OrderView {\n        return new OrderView($id, 'patched', $theme);\n    }\n}\n",
            ),
        ],
    };

    const JAVA: Self = Self {
        extension: "java",
        program: "java",
        version_flag: "--version",
        files: &[
            (
                "server/models/OrderView.java",
                "record OrderView(long id, String tenant, String theme) {}\n",
            ),
            (
                "server/controllers/OrdersController.java",
                "import models.OrderView;\n\n@RequestRoute(\"/orders\")\n@Public\nclass OrdersController {\n    @PutRoute(\"/{id}\")\n    OrderView putOrder(@FromPath long id, @FromHeader(\"x-tenant\") String tenant, @FromCookie(\"theme\") String theme) {\n        return new OrderView(id, tenant, theme);\n    }\n\n    @PatchRoute(\"/{id}\")\n    OrderView patchOrder(@FromPath long id, @FromCookie(\"theme\") String theme) {\n        return new OrderView(id, \"patched\", theme);\n    }\n}\n",
            ),
        ],
    };

    const GROOVY: Self = Self {
        extension: "groovy",
        program: "groovyc",
        version_flag: "--version",
        files: &[
            (
                "server/models/OrderView.groovy",
                "record OrderView(long id, String tenant, String theme) {}\n",
            ),
            (
                "server/controllers/OrdersController.groovy",
                "import models.OrderView\n\n@RequestRoute('/orders')\n@Public\nclass OrdersController {\n    @PutRoute(\"/{id}\")\n    OrderView putOrder(@FromPath long id, @FromHeader('x-tenant') String tenant, @FromCookie('theme') String theme) {\n        return new OrderView(id, tenant, theme)\n    }\n\n    @PatchRoute(\"/{id}\")\n    OrderView patchOrder(@FromPath long id, @FromCookie('theme') String theme) {\n        return new OrderView(id, 'patched', theme)\n    }\n}\n",
            ),
        ],
    };

    const KOTLIN: Self = Self {
        extension: "kt",
        program: "kotlinc",
        version_flag: "-version",
        files: &[
            (
                "server/models/OrderView.kt",
                "data class OrderView(val id: Long, val tenant: String, val theme: String)\n",
            ),
            (
                "server/controllers/OrdersController.kt",
                "import models.OrderView\n\n@RequestRoute(\"/orders\")\n@Public\nclass OrdersController {\n    @PutRoute(\"/{id}\")\n    fun putOrder(@FromPath id: Long, @FromHeader(\"x-tenant\") tenant: String, @FromCookie(\"theme\") theme: String): OrderView {\n        return OrderView(id, tenant, theme)\n    }\n\n    @PatchRoute(\"/{id}\")\n    fun patchOrder(@FromPath id: Long, @FromCookie(\"theme\") theme: String): OrderView {\n        return OrderView(id, \"patched\", theme)\n    }\n}\n",
            ),
        ],
    };

    const CSHARP: Self = Self {
        extension: "cs",
        program: "dotnet",
        version_flag: "--version",
        files: &[
            (
                "server/models/OrderView.cs",
                "public record OrderView(long Id, string Tenant, string Theme);\n",
            ),
            (
                "server/controllers/OrdersController.cs",
                "using Models;\n\n[RequestRoute(\"/orders\")]\n[Public]\nclass OrdersController {\n    [PutRoute(\"/{id}\")]\n    public OrderView PutOrder([FromPath] long id, [FromHeader(\"x-tenant\")] string tenant, [FromCookie(\"theme\")] string theme) {\n        return new OrderView(id, tenant, theme);\n    }\n\n    [PatchRoute(\"/{id}\")]\n    public OrderView PatchOrder([FromPath] long id, [FromCookie(\"theme\")] string theme) {\n        return new OrderView(id, \"patched\", theme);\n    }\n}\n",
            ),
        ],
    };

    const FSHARP: Self = Self {
        extension: "fs",
        program: "dotnet",
        version_flag: "--version",
        files: &[
            (
                "server/models/OrderView.fs",
                "type OrderView = { Id: int64; Tenant: string; Theme: string }\n",
            ),
            (
                "server/controllers/OrdersController.fs",
                "open Models\n\n[<RequestRoute(\"/orders\")>]\n[<Public>]\ntype OrdersController() =\n    [<PutRoute(\"/{id}\")>]\n    member this.PutOrder([<FromPath>] id: int64, [<FromHeader(\"x-tenant\")>] tenant: string, [<FromCookie(\"theme\")>] theme: string) : OrderView =\n        { Id = id; Tenant = tenant; Theme = theme }\n\n    [<PatchRoute(\"/{id}\")>]\n    member this.PatchOrder([<FromPath>] id: int64, [<FromCookie(\"theme\")>] theme: string) : OrderView =\n        { Id = id; Tenant = \"patched\"; Theme = theme }\n",
            ),
        ],
    };

    const RUST: Self = Self {
        extension: "rs",
        program: "rustc",
        version_flag: "--version",
        files: &[
            (
                "server/models/OrderView.rs",
                "pub struct OrderView {\n    pub id: i64,\n    pub tenant: String,\n    pub theme: String,\n}\n",
            ),
            (
                "server/controllers/OrdersController.rs",
                "use crate::models::OrderView;\n\n#[RequestRoute(\"/orders\")]\n#[Public]\nstruct OrdersController {}\nimpl OrdersController {\n    fn new() -> Self { Self {} }\n\n    #[PutRoute(\"/{id}\")]\n    fn put_order(&self, #[FromPath] id: i64, #[FromHeader(\"x-tenant\")] tenant: String, #[FromCookie(\"theme\")] theme: String) -> OrderView {\n        OrderView { id, tenant, theme }\n    }\n\n    #[PatchRoute(\"/{id}\")]\n    fn patch_order(&self, #[FromPath] id: i64, #[FromCookie(\"theme\")] theme: String) -> OrderView {\n        OrderView { id, tenant: String::from(\"patched\"), theme }\n    }\n}\n",
            ),
        ],
    };

    const DART: Self = Self {
        extension: "dart",
        program: "dart",
        version_flag: "--version",
        files: &[
            (
                "server/models/OrderView.dart",
                "class OrderView {\n  final int id;\n  final String tenant;\n  final String theme;\n\n  const OrderView(this.id, this.tenant, this.theme);\n}\n",
            ),
            (
                "server/controllers/OrdersController.dart",
                "import 'models/OrderView.dart';\n\n@RequestRoute(\"/orders\")\n@Public\nclass OrdersController {\n  @PutRoute(\"/{id}\")\n  OrderView putOrder(@FromPath int id, @FromHeader(\"x-tenant\") String tenant, @FromCookie(\"theme\") String theme) {\n    return OrderView(id, tenant, theme);\n  }\n\n  @PatchRoute(\"/{id}\")\n  OrderView patchOrder(@FromPath int id, @FromCookie(\"theme\") String theme) {\n    return OrderView(id, 'patched', theme);\n  }\n}\n",
            ),
        ],
    };

    const GO: Self = Self {
        extension: "go",
        program: "go",
        version_flag: "version",
        files: &[
            (
                "server/models/OrderView.go",
                "type OrderView struct {\n\tId     int64  `json:\"id\"`\n\tTenant string `json:\"tenant\"`\n\tTheme  string `json:\"theme\"`\n}\n",
            ),
            (
                "server/controllers/OrdersController.go",
                "// @RequestRoute(\"/orders\")\n// @Public\ntype OrdersController struct {\n}\n\n// @PutRoute(\"/{id}\")\nfunc (controller *OrdersController) PutOrder(\n// @FromPath\n id int64, \n// @FromHeader(\"x-tenant\")\n tenant string, \n// @FromCookie(\"theme\")\n theme string) OrderView {\n\treturn OrderView{Id: id, Tenant: tenant, Theme: theme}\n}\n\n// @PatchRoute(\"/{id}\")\nfunc (controller *OrdersController) PatchOrder(\n// @FromPath\n id int64, \n// @FromCookie(\"theme\")\n theme string) OrderView {\n\treturn OrderView{Id: id, Tenant: \"patched\", Theme: theme}\n}\n",
            ),
        ],
    };

    const LUA: Self = Self {
        extension: "lua",
        program: "lua",
        version_flag: "-v",
        files: &[
            (
                "server/models/OrderView.lua",
                "---@class OrderView\n---@field id integer\n---@field tenant string\n---@field theme string\nOrderView = {}\n",
            ),
            (
                "server/controllers/OrdersController.lua",
                "-- @RequestRoute(\"/orders\")\n-- @Public\n---@class OrdersController\nOrdersController = {}\n\n---@param id integer\n---@param tenant string\n---@param theme string\n---@return OrderView\n-- @PutRoute(\"/{id}\")\nfunction OrdersController:put_order(\n  -- @FromPath\n  id,\n  -- @FromHeader(\"x-tenant\")\n  tenant,\n  -- @FromCookie(\"theme\")\n  theme\n)\n  return { id = id, tenant = tenant, theme = theme }\nend\n\n---@param id integer\n---@param theme string\n---@return OrderView\n-- @PatchRoute(\"/{id}\")\nfunction OrdersController:patch_order(\n  -- @FromPath\n  id,\n  -- @FromCookie(\"theme\")\n  theme\n)\n  return { id = id, tenant = \"patched\", theme = theme }\nend\n",
            ),
        ],
    };

    const CPP: Self = Self {
        extension: "hpp",
        program: "clang++",
        version_flag: "--version",
        files: &[
            (
                "server/models/OrderView.hpp",
                "#pragma once\n#include <cstdint>\n#include <string>\n\nstruct OrderView {\n    std::int64_t id;\n    std::string tenant;\n    std::string theme;\n};\n",
            ),
            (
                "server/controllers/OrdersController.hpp",
                "#pragma once\n#include <cstdint>\n#include <string>\n#include \"../models/OrderView.hpp\"\n\nclass [[RequestRoute(\"/orders\")]] [[Public]] OrdersController {\npublic:\n    [[PutRoute(\"/{id}\")]]\n    OrderView putOrder([[FromPath]] std::int64_t id, [[FromHeader(\"x-tenant\")]] std::string tenant, [[FromCookie(\"theme\")]] std::string theme) {\n        return OrderView{id, tenant, theme};\n    }\n\n    [[PatchRoute(\"/{id}\")]]\n    OrderView patchOrder([[FromPath]] std::int64_t id, [[FromCookie(\"theme\")]] std::string theme) {\n        return OrderView{id, \"patched\", theme};\n    }\n};\n",
            ),
        ],
    };

    const COOKIE: &'static str = "Cookie: size=large; theme=dark\r\n";
    const TENANT: &'static str = "X-Tenant: north\r\n";

    fn available(&self) -> bool {
        crate::support::program_probe::ProgramProbe::answers(self.program, self.version_flag)
    }

    fn write(&self, root: &Path) {
        for (relative, source) in self.files {
            let path = root.join(relative);
            fs::create_dir_all(path.parent().expect("parent")).expect("directories");
            fs::write(path, source).expect("source");
        }
    }

    async fn check(&self) {
        if !self.available() {
            eprintln!(
                "skipping {}: {} is unavailable",
                self.extension, self.program
            );
            return;
        }
        let root = tempfile::tempdir().expect("project");
        self.write(root.path());
        let server = SourcesServer::start(root.path(), self.extension).await;
        let headers = format!("{}{}", Self::TENANT, Self::COOKIE);
        let put = server.call("PUT", "/orders/7", &headers).await;
        let patched = server.call("PATCH", "/orders/8", Self::COOKIE).await;
        server.stop().await;
        assert_eq!(
            [put, patched],
            [
                serde_json::json!({"id": 7, "tenant": "north", "theme": "dark"}),
                serde_json::json!({"id": 8, "tenant": "patched", "theme": "dark"}),
            ],
            "{}",
            self.extension
        );
    }
}

struct SourcesServer {
    extension: &'static str,
    port: u16,
    stop: tokio::sync::oneshot::Sender<()>,
    running: tokio::task::JoinHandle<Result<(), tachyon_core::Failure>>,
}

impl SourcesServer {
    async fn start(root: &Path, extension: &'static str) -> Self {
        let project = ProjectDiscovery::discover(root)
            .unwrap_or_else(|failure| panic!("{extension}: {failure}"));
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
            extension,
            port,
            stop,
            running,
        }
    }

    async fn call(&self, method: &str, path: &str, headers: &str) -> serde_json::Value {
        let mut stream = TcpStream::connect(("127.0.0.1", self.port))
            .await
            .expect("connect");
        let raw = format!(
            "{method} {path} HTTP/1.1\r\nHost: localhost\r\n{headers}Content-Length: 0\r\nConnection: close\r\n\r\n"
        );
        stream.write_all(raw.as_bytes()).await.expect("write");
        let mut response = Vec::new();
        tokio::time::timeout(Duration::from_mins(5), stream.read_to_end(&mut response))
            .await
            .expect("answer within the deadline")
            .expect("read");
        let response = String::from_utf8_lossy(&response).into_owned();
        let (head, body) = response.split_once("\r\n\r\n").expect("HTTP body");
        assert!(
            head.starts_with("HTTP/1.1 200"),
            "{}: {response}",
            self.extension
        );
        serde_json::from_str(body).unwrap_or_else(|_| panic!("{}: {response}", self.extension))
    }

    async fn stop(self) {
        self.stop.send(()).expect("stop");
        self.running.await.expect("server task").expect("shutdown");
    }
}

#[tokio::test]
async fn a_javascript_route_reads_its_path_header_and_cookie() {
    SourcesProject::JAVASCRIPT.check().await;
}

#[tokio::test]
async fn a_typescript_route_reads_its_path_header_and_cookie() {
    SourcesProject::TYPESCRIPT.check().await;
}

#[tokio::test]
async fn a_python_route_reads_its_path_header_and_cookie() {
    SourcesProject::PYTHON.check().await;
}

#[tokio::test]
async fn a_ruby_route_reads_its_path_header_and_cookie() {
    SourcesProject::RUBY.check().await;
}

#[tokio::test]
async fn a_php_route_reads_its_path_header_and_cookie() {
    SourcesProject::PHP.check().await;
}

#[tokio::test]
async fn a_java_route_reads_its_path_header_and_cookie() {
    SourcesProject::JAVA.check().await;
}

#[tokio::test]
async fn a_groovy_route_reads_its_path_header_and_cookie() {
    SourcesProject::GROOVY.check().await;
}

#[tokio::test]
async fn a_kotlin_route_reads_its_path_header_and_cookie() {
    SourcesProject::KOTLIN.check().await;
}

#[tokio::test]
async fn a_csharp_route_reads_its_path_header_and_cookie() {
    SourcesProject::CSHARP.check().await;
}

#[tokio::test]
async fn an_fsharp_route_reads_its_path_header_and_cookie() {
    SourcesProject::FSHARP.check().await;
}

#[tokio::test]
async fn a_rust_route_reads_its_path_header_and_cookie() {
    SourcesProject::RUST.check().await;
}

#[tokio::test]
async fn a_dart_route_reads_its_path_header_and_cookie() {
    SourcesProject::DART.check().await;
}

#[tokio::test]
async fn a_go_route_reads_its_path_header_and_cookie() {
    SourcesProject::GO.check().await;
}

#[tokio::test]
async fn a_lua_route_reads_its_path_header_and_cookie() {
    SourcesProject::LUA.check().await;
}

#[tokio::test]
async fn a_cpp_route_reads_its_path_header_and_cookie() {
    SourcesProject::CPP.check().await;
}
