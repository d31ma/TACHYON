#![allow(clippy::expect_used)]

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;
use tachyon_contracts::{HandlerRequest, HttpMethod};
use tachyon_core::{
    ApplicationConfig, BuildOptions, DevServer, DevServerOptions, HandlerCancellation,
    HandlerSource, HandlerSupervisor, HandlerSupervisorOptions, ProjectDiscovery, WebCompiler,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

struct ConfigurationFixture;

impl ConfigurationFixture {
    const CONFIGURATION: &str = r#"{
  "server": {"host": "127.0.0.1", "port": 8081},
  "datasource": {"driver": "sqlite", "url": "sqlite::memory:"},
  "catalog": {"size": 20},
  "tenant": {"owner": "reader"}
}"#;

    const JAVASCRIPT: &str = r"
export class Handler {
  static GET() { return JSON.parse(process.env.YON_APPLICATION_JSON) }
}
";

    const CONTROLLER: &str = r#"
@RequestRoute("/configuration")
@Public
class ConfigurationController {
  @GetRoute
  getConfiguration() { return JSON.parse(process.env.YON_APPLICATION_JSON) }
}
"#;

    const PYTHON: &str = r#"
import json
import os
class Handler:
    @staticmethod
    def GET(request):
        return json.loads(os.environ["YON_APPLICATION_JSON"])
"#;
}

fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    fs::write(path, contents).expect("fixture");
}

fn configuration(root: &Path) -> ApplicationConfig {
    write(
        root,
        "server/application.json",
        ConfigurationFixture::CONFIGURATION,
    );
    write(
        root,
        "server/application-test.json",
        r#"{"catalog":{"size":30},"tenant":{"owner":"profile-reader"}}"#,
    );
    write(
        root,
        ".env",
        "YON_TENANT_SECRET=private-configuration-canary",
    );
    let mut environment = BTreeMap::from([
        ("YON_PROFILE".into(), "test".into()),
        (
            "YON_TEST_DATABASE".into(),
            "postgres://owned.invalid/catalog".into(),
        ),
        ("YON_CATALOG_SIZE".into(), "40".into()),
    ]);
    let application = ApplicationConfig::load_with_environment(root, &environment)
        .expect("resolved private configuration");
    write(root, "server/application.json", "{}");
    write(root, "server/application-test.json", "{}");
    write(root, ".env", "YON_TENANT_SECRET=changed");
    environment.insert("YON_TEST_DATABASE".into(), "changed".into());
    application
}

fn assert_configuration(value: &serde_json::Value) {
    assert_eq!(value["yon"]["datasource"]["driver"], "sqlite");
    assert_eq!(value["yon"]["datasource"]["url"], "sqlite::memory:");
    assert_eq!(value["yon"]["tenant"]["owner"], "profile-reader");
    assert_eq!(value["yon"]["catalog"]["size"], 30);
    assert_eq!(
        value["yon"]["tenant"]["secret"],
        "private-configuration-canary"
    );
}

fn source(root: &Path, extension: &str, contents: &str) -> HandlerSource {
    let relative = format!("server/workers/configuration.{extension}");
    write(root, &relative, contents);
    HandlerSource::discover(root, &relative).expect("captured handler")
}

fn controller(root: &Path) {
    write(
        root,
        "server/controllers/ConfigurationController.js",
        ConfigurationFixture::CONTROLLER,
    );
}

fn supervisor(application: ApplicationConfig, extension: &str) -> HandlerSupervisor {
    let mut options = HandlerSupervisorOptions {
        default_timeout: Duration::from_mins(2),
        application,
        ..HandlerSupervisorOptions::default()
    };
    if extension == "ts" {
        options.runtimes.javascript = "bun".into();
    }
    HandlerSupervisor::new(options).expect("supervisor")
}

fn available(program: &str) -> bool {
    Command::new(program)
        .arg(match program {
            "kotlinc" => "-version",
            "go" => "version",
            _ => "--version",
        })
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

async fn exercise(extension: &str, contents: &str) {
    let root = tempfile::tempdir().expect("project");
    let original_environment = std::env::var_os("YON_APPLICATION_JSON");
    let application = configuration(root.path());
    let source = source(root.path(), extension, contents);
    let response = supervisor(application, extension)
        .invoke(
            &source,
            &HandlerRequest::route("application_config", "/configuration", HttpMethod::Get),
            &HandlerCancellation::default(),
        )
        .await
        .expect("configured handler response");
    assert!(response.error.is_none(), "{:?}", response.error);
    assert_eq!(response.status, 200);
    let value = serde_json::from_str(&response.body.expect("JSON body").data).expect("JSON");
    assert_configuration(&value);
    assert_eq!(
        std::env::var_os("YON_APPLICATION_JSON"),
        original_environment
    );
}

#[tokio::test]
async fn javascript_receives_the_private_immutable_configuration() {
    exercise("js", ConfigurationFixture::JAVASCRIPT).await;
}

#[tokio::test]
async fn application_properties_feed_the_existing_typed_binding_contract() {
    exercise(
        "js",
        r#"
@Settings({"prefix":"YON_CATALOG_"})
export class Handler {
  @Min(1)
  @Max(100)
  /** @type {integer} */
  size;
  static GET() {
    const application = JSON.parse(process.env.YON_APPLICATION_JSON);
    const settings = Behaviors.properties(Handler, application);
    if (settings.size !== 30) throw new Error('Typed application binding failed');
    return application;
  }
}
"#,
    )
    .await;
}

#[tokio::test]
async fn python_receives_the_private_immutable_configuration() {
    exercise("py", ConfigurationFixture::PYTHON).await;
}

#[tokio::test]
async fn typescript_receives_the_private_immutable_configuration() {
    if !available("bun") {
        eprintln!("TypeScript configuration execution requires Bun.");
        return;
    }
    exercise("ts", ConfigurationFixture::JAVASCRIPT).await;
}

#[tokio::test]
async fn php_receives_the_private_immutable_configuration() {
    if !available("php-cgi") {
        eprintln!("PHP configuration execution requires PHP.");
        return;
    }
    exercise(
        "php",
        r#"<?php
class Handler {
    public static function GET(): void {
        header('Content-Type: application/json');
        echo getenv("YON_APPLICATION_JSON");
    }
}
"#,
    )
    .await;
}

#[tokio::test]
async fn java_receives_the_private_immutable_configuration() {
    if !available("java") {
        eprintln!("Java configuration execution requires a JDK.");
        return;
    }
    exercise(
        "java",
        r#"
class Handler {
    static void GET(com.sun.net.httpserver.HttpExchange exchange) throws java.io.IOException {
        byte[] body = System.getenv("YON_APPLICATION_JSON").getBytes(java.nio.charset.StandardCharsets.UTF_8);
        exchange.getResponseHeaders().add("content-type", "application/json");
        exchange.sendResponseHeaders(200, body.length);
        exchange.getResponseBody().write(body);
    }
}
"#,
    )
    .await;
}

#[tokio::test]
async fn kotlin_receives_the_private_immutable_configuration() {
    if !available("kotlinc") {
        eprintln!("Kotlin configuration execution requires Kotlin.");
        return;
    }
    exercise(
        "kt",
        r#"
object Handler {
    @JvmStatic fun GET(exchange: com.sun.net.httpserver.HttpExchange) {
        val body = System.getenv("YON_APPLICATION_JSON").toByteArray()
        exchange.responseHeaders.add("content-type", "application/json")
        exchange.sendResponseHeaders(200, body.size.toLong())
        exchange.responseBody.write(body)
    }
}
"#,
    )
    .await;
}

#[tokio::test]
async fn csharp_receives_the_private_immutable_configuration() {
    if !available("dotnet") {
        eprintln!("C# configuration execution requires .NET.");
        return;
    }
    exercise("cs", r#"
class Handler {
    public static HttpResponseMessage GET(HttpRequestMessage request) =>
        new HttpResponseMessage(System.Net.HttpStatusCode.OK) { Content = new StringContent(System.Environment.GetEnvironmentVariable("YON_APPLICATION_JSON") ?? "null", System.Text.Encoding.UTF8, "application/json") };
}
"#).await;
}

#[tokio::test]
async fn rust_receives_the_private_immutable_configuration() {
    exercise(
        "rs",
        r#"
struct Handler;
impl Handler {
    fn GET() -> http::Response<Vec<u8>> {
        let body = std::env::var("YON_APPLICATION_JSON").expect("application config");
        http::Response::builder()
            .header("content-type", "application/json")
            .body(body.into_bytes())
            .unwrap_or_default()
    }
}
"#,
    )
    .await;
}

#[tokio::test]
async fn go_receives_the_private_immutable_configuration() {
    if !available("go") {
        let required = cfg!(unix) && std::env::var_os("CI").is_some();
        assert!(
            !required,
            "go is required for the Go configuration test in CI"
        );
        eprintln!("Go configuration execution requires Go.");
        return;
    }
    exercise(
        "go",
        r#"
import (
	"io"
	"net/http"
	"os"
	"strings"
)

type Handler struct{}

func (Handler) GET(request *http.Request) *http.Response {
	body := os.Getenv("YON_APPLICATION_JSON")
	return &http.Response{
		StatusCode: http.StatusOK,
		Header:     http.Header{"Content-Type": {"application/json"}},
		Body:       io.NopCloser(strings.NewReader(body)),
	}
}
"#,
    )
    .await;
}

#[tokio::test]
async fn streaming_handlers_receive_the_same_configuration() {
    for (extension, contents) in [
        (
            "js",
            r"
export class Handler {
  static async *GET() { yield JSON.parse(process.env.YON_APPLICATION_JSON) }
}
",
        ),
        (
            "py",
            r#"
import json
import os
class Handler:
    @staticmethod
    def GET(request):
        yield json.loads(os.environ["YON_APPLICATION_JSON"])
"#,
        ),
    ] {
        assert_streaming_configuration(extension, contents).await;
    }
}

async fn assert_streaming_configuration(extension: &str, contents: &str) {
    let root = tempfile::tempdir().expect("project");
    let application = configuration(root.path());
    let source = source(root.path(), extension, contents);
    let supervisor = supervisor(application, extension);
    let (sender, mut receiver) = tokio::sync::mpsc::channel(4);
    supervisor
        .invoke_streaming(
            &source,
            &HandlerRequest::route("stream_configuration", "/configuration", HttpMethod::Get),
            sender,
        )
        .await
        .expect("configured stream");
    let event = receiver.recv().await.expect("configuration event");
    assert_configuration(&serde_json::from_str(&event.data).expect("JSON event"));
    assert!(receiver.recv().await.is_none());
}

fn assert_private_files_absent(directory: &Path) {
    for entry in fs::read_dir(directory).expect("output directory") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            assert_private_files_absent(&path);
            continue;
        }
        assert_ne!(path.file_name().expect("name"), ".env");
        let bytes = fs::read(&path).expect("artifact");
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            !text.contains("private-configuration-canary"),
            "{}",
            path.display()
        );
        assert!(
            !text.contains("YON_BUILD_MUST_NOT_RESOLVE"),
            "{}",
            path.display()
        );
    }
}

#[test]
fn tac_build_does_not_resolve_or_publish_server_configuration() {
    let root = tempfile::tempdir().expect("project");
    write(root.path(), "client/pages/tac.html", "<app-view />");
    write(
        root.path(),
        "client/components/app/view/tac.html",
        "<main>Public page</main>",
    );
    write(
        root.path(),
        "client/manifest.json",
        r#"{"name":"Public","tac":{"label":"Public"}}"#,
    );
    write(
        root.path(),
        "server/application.json",
        r#"{"datasource":{"url":"postgres://owned.invalid/catalog"}}"#,
    );
    write(
        root.path(),
        ".env",
        "YON_BUILD_MUST_NOT_RESOLVE=private-configuration-canary",
    );
    write(
        root.path(),
        "server/application-production.json",
        r#"{"redis":{"password":"private-configuration-canary"}}"#,
    );
    let project =
        ProjectDiscovery::discover(root.path()).expect("discovery ignores runtime secrets");
    let output = WebCompiler::build_project(&project, &BuildOptions::default())
        .expect("Tac build never resolves server configuration");
    assert_eq!(output.route_count(), 1);
    assert_private_files_absent(output.output_directory());
}

#[tokio::test]
async fn explicit_library_server_options_supersede_invalid_file_endpoints() {
    let root = tempfile::tempdir().expect("project");
    write(root.path(), "client/pages/tac.html", "<app-view />");
    write(
        root.path(),
        "client/components/app/view/tac.html",
        "<main>Public page</main>",
    );
    write(
        root.path(),
        "server/application.json",
        r#"{"server":{"host":"invalid-host","port":"invalid-port"}}"#,
    );
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
    .expect("explicit library options replace invalid endpoints");
    assert!(server.address().ip().is_loopback());
    assert_ne!(server.address().port(), 0);
}

#[tokio::test]
async fn http_dispatch_uses_the_supplied_configuration_snapshot() {
    let root = tempfile::tempdir().expect("project");
    let application = configuration(root.path());
    write(root.path(), "client/pages/tac.html", "<app-view />");
    write(
        root.path(),
        "client/components/app/view/tac.html",
        "<main>Public page</main>",
    );
    controller(root.path());
    write(root.path(), "server/application.json", "{}");
    write(root.path(), ".env", "not valid dotenv");
    let server = DevServer::bind_with_application(
        root.path(),
        &DevServerOptions {
            port: 0,
            watch: false,
            ..DevServerOptions::default()
        },
        application,
    )
    .await
    .expect("server with supplied configuration");
    let address = server.address();
    let (stop, shutdown) = tokio::sync::oneshot::channel();
    let running = tokio::spawn(server.run_until(async {
        let _ = shutdown.await;
    }));
    let response = http_get(address, "/configuration").await;
    assert_private_paths_hidden(address).await;
    stop.send(()).expect("stop server");
    running.await.expect("server task").expect("shutdown");
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    let (_, body) = response.split_once("\r\n\r\n").expect("HTTP body");
    assert_configuration(&serde_json::from_str(body).expect("HTTP JSON"));
}

async fn assert_private_paths_hidden(address: std::net::SocketAddr) {
    for path in [
        "/server/application.json",
        "/server/application-test.json",
        "/application.json",
        "/.env",
    ] {
        let private = http_get(address, path).await;
        assert!(private.starts_with("HTTP/1.1 404"), "{private}");
        assert!(!private.contains("sqlite::memory:"));
    }
}

#[cfg(unix)]
#[tokio::test]
async fn library_binding_reads_configuration_from_the_retained_project_capability() {
    let workspace = tempfile::tempdir().expect("workspace");
    let project_root = workspace.path().join("project");
    write(&project_root, "client/pages/tac.html", "<app-view />");
    write(
        &project_root,
        "client/components/app/view/tac.html",
        "<main>Public page</main>",
    );
    write(
        &project_root,
        "server/application.json",
        r#"{"datasource":{"driver":"sqlite","url":"sqlite::memory:"},"catalog":{"origin":"retained"}}"#,
    );
    controller(&project_root);
    let project = ProjectDiscovery::discover(&project_root).expect("captured project");
    fs::rename(&project_root, workspace.path().join("retained-project"))
        .expect("move authored project");
    write(
        &project_root,
        "server/application.json",
        r#"{"datasource":{"driver":"sqlite","url":"sqlite::memory:"},"catalog":{"origin":"replacement"}}"#,
    );
    let server = DevServer::bind_project(
        &project,
        &DevServerOptions {
            port: 0,
            watch: false,
            ..DevServerOptions::default()
        },
    )
    .await
    .expect("bind with retained project authority");
    let address = server.address();
    let (stop, shutdown) = tokio::sync::oneshot::channel();
    let running = tokio::spawn(server.run_until(async {
        let _ = shutdown.await;
    }));
    let response = http_get(address, "/configuration").await;
    stop.send(()).expect("stop server");
    running.await.expect("server task").expect("shutdown");
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    let (_, body) = response.split_once("\r\n\r\n").expect("HTTP body");
    let value: serde_json::Value = serde_json::from_str(body).expect("HTTP JSON");
    assert_eq!(value["yon"]["catalog"]["origin"], "retained");
}

async fn http_get(address: std::net::SocketAddr, path: &str) -> String {
    let mut stream = tokio::net::TcpStream::connect(address)
        .await
        .expect("connect");
    stream
        .write_all(
            format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        )
        .await
        .expect("request");
    let mut response = String::new();
    tokio::time::timeout(
        Duration::from_secs(30),
        stream.read_to_string(&mut response),
    )
    .await
    .expect("bounded HTTP response")
    .expect("read response");
    response
}
