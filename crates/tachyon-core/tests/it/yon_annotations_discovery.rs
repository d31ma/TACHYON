#![allow(clippy::expect_used)]

use crate::support::yon_layout;

use std::fs;
use std::path::Path;
use std::time::Duration;
use tachyon_core::{DevServer, DevServerOptions, Project, ProjectDiscovery};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpStream;

fn write(root: &Path, path: &str, source: &str) {
    let destination = root.join(path);
    fs::create_dir_all(destination.parent().expect("parent")).expect("directory");
    fs::write(destination, source).expect("source");
}

async fn get(port: u16, path: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("connect");
    stream
        .write_all(
            format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        )
        .await
        .expect("request");
    let mut bytes = Vec::new();
    tokio::time::timeout(Duration::from_secs(20), stream.read_to_end(&mut bytes))
        .await
        .expect("bounded response")
        .expect("response");
    String::from_utf8(bytes).expect("UTF-8 response")
}

#[tokio::test]
async fn annotated_controller_preserves_snapshot_roles_and_annotation_routing() {
    let root = tempfile::tempdir().expect("project");
    write_snapshot_project(root.path());
    let project = ProjectDiscovery::discover(root.path()).expect("annotation discovery");
    replace_authored_sources(root.path());
    assert_snapshot_response(&project).await;
}

#[tokio::test]
async fn controller_preserves_snapshot_imports_and_annotation_routing() {
    let root = tempfile::tempdir().expect("project");
    write_snapshot_import_project(root.path());
    let project = ProjectDiscovery::discover(root.path()).expect("importing controller discovery");
    write(
        root.path(),
        "server/shared/catalog.js",
        "throw new Error('reopened helper');",
    );
    write(
        root.path(),
        "server/controllers/ItemsController.js",
        "throw new Error('reopened route');",
    );
    assert_snapshot_response(&project).await;
}

#[tokio::test]
async fn service_shared_imports_use_the_captured_source_snapshot() {
    let root = tempfile::tempdir().expect("project");
    write(
        root.path(),
        "server/shared/catalog.js",
        "export class Catalog { static label = 'captured'; }",
    );
    write(
        root.path(),
        "server/services/CatalogService.js",
        "import { Catalog } from '../shared/catalog.js';\n@Service\nclass CatalogService { label() { return Catalog.label; } }\n",
    );
    write(
        root.path(),
        "server/controllers/ItemsController.js",
        "@RequestRoute(\"/items/{id}\")\n@Public\n@Needs(CatalogService)\nclass ItemsController {\n constructor(catalog) { this.catalog = catalog; }\n @GetRoute\n get(@FromPath id) { return {id, label: this.catalog.label()}; }\n}\n",
    );
    let project = ProjectDiscovery::discover(root.path()).expect("shared service discovery");
    write(
        root.path(),
        "server/shared/catalog.js",
        "throw new Error('reopened shared helper');",
    );
    write(
        root.path(),
        "server/services/CatalogService.js",
        "throw new Error('reopened service');",
    );
    assert_snapshot_response(&project).await;
}

async fn assert_snapshot_response(project: &Project) {
    let server = DevServer::bind_project(
        project,
        &DevServerOptions {
            port: 0,
            watch: false,
            ..DevServerOptions::default()
        },
    )
    .await
    .expect("bind owned project");
    let port = server.address().port();
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let running = tokio::spawn(server.run_until(async {
        let _ = stopped.await;
    }));
    let response = get(port, "/items/42").await;
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    assert!(
        response.contains("\"id\":\"42\"") && response.contains("\"label\":\"captured\""),
        "{response}"
    );
    assert!(get(port, "/Items").await.starts_with("HTTP/1.1 404"));
    assert!(
        get(port, "/items/42/helper")
            .await
            .starts_with("HTTP/1.1 404")
    );
    stop.send(()).expect("shutdown");
    tokio::time::timeout(Duration::from_secs(20), running)
        .await
        .expect("bounded shutdown")
        .expect("server task")
        .expect("server shutdown");
}

#[test]
fn invalid_provider_graph_fails_without_running_application_code() {
    let root = tempfile::tempdir().expect("project");
    yon_layout::write_fixture(
        root.path(),
        "server/controllers/ItemsController.js",
        r#"
@RequestRoute("/")
@Public
@Needs(Missing)
class ItemsController {
  static _loaded = (() => { throw new Error('discovery executed application code'); })();
  constructor(missing) { this.missing = missing; }
  @GetRoute()
  get() { return []; }
}
"#,
    );
    let failure = ProjectDiscovery::discover(root.path()).expect_err("unresolved provider");
    assert!(
        failure
            .to_string()
            .contains("Unresolved dependency Missing"),
        "{failure}"
    );
    assert!(!root.path().join("dist").exists());
}

#[tokio::test]
async fn discovery_and_bundle_never_construct_annotated_providers() {
    let root = tempfile::tempdir().expect("project");
    write(root.path(), "client/pages/tac.html", "<app-view />");
    write(
        root.path(),
        "client/components/app/view/tac.html",
        "<main>Client only</main>",
    );
    yon_layout::write_fixture(
        root.path(),
        "server/controllers/ItemsController.js",
        r#"
@RequestRoute("/")
@Public
class ItemsController {
  static _loaded = (() => { throw new Error('must only execute on invocation'); })();
  constructor() { throw new Error('must not construct during build'); }
  @GetRoute()
  get() { return []; }
}
"#,
    );
    let project = ProjectDiscovery::discover(root.path()).expect("inert discovery");
    tachyon_core::WebCompiler::build_project_async(
        &project,
        &tachyon_core::BuildOptions::default(),
    )
    .await
    .expect("inert bundle");
    assert!(root.path().join("dist/index.html").is_file());
}

fn write_snapshot_project(root: &Path) {
    write(root, "client/pages/tac.html", "<app-view />");
    write(
        root,
        "client/components/app/view/tac.html",
        "<main>Client only</main>",
    );
    yon_layout::write_fixture(
        root,
        "server/controllers/ItemsController.js",
        r#"
@Service()
class CatalogService {
  title() { return 'captured'; }
}
@RequestRoute("/items/{id}")
@Public
@Needs(CatalogService)
class ItemsController {
  constructor(catalog) { this.catalog = catalog; }
  @GetRoute()
  get(@FromPath id) { return {id, label: this.catalog.title()}; }
  _helper() { throw new Error('helper is not a route'); }
}
"#,
    );
}

fn replace_authored_sources(root: &Path) {
    write(
        root,
        "server/controllers/ItemsController.js",
        "throw new Error('reopened route');",
    );
    write(
        root,
        "server/services/CatalogService.js",
        "throw new Error('reopened role module');",
    );
    write(
        root,
        "server/controllers/ItemsController.module.json",
        "new unsupported manifest must not affect the captured project",
    );
}

fn write_snapshot_import_project(root: &Path) {
    write(
        root,
        "server/shared/catalog.js",
        "export class Catalog { static label = 'captured'; }",
    );
    write(
        root,
        "server/controllers/ItemsController.js",
        r#"
import { Catalog } from '../shared/catalog.js';
@RequestRoute("/items/{id}")
@Public
export class ItemsController {
  @GetRoute()
  get(@FromPath id) { return {id, label: Catalog.label}; }
  _helper() { throw new Error('helper is not a route'); }
}
"#,
    );
}
