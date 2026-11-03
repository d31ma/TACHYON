#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;
use std::time::Duration;
use tachyon_contracts::{HandlerRequest, HttpMethod};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
};

struct Layout;

impl Layout {
    const ROLES: [(&'static str, &'static str); 18] = [
        ("@RequestRoute", "controllers"),
        ("@request_route", "controllers"),
        ("#[RequestRoute", "controllers"),
        ("[RequestRoute", "controllers"),
        ("@Service", "services"),
        ("@service", "services"),
        ("#[Service", "services"),
        ("[Service", "services"),
        ("#[yon::service", "services"),
        ("@yon::service", "services"),
        ("@yon::Service", "services"),
        ("@Store", "stores"),
        ("@store", "stores"),
        ("#[Store", "stores"),
        ("[Store", "stores"),
        ("#[yon::store", "stores"),
        ("@yon::store", "stores"),
        ("@yon::Store", "stores"),
    ];

    fn folder(line: &str) -> Option<&'static str> {
        let line = line.strip_prefix("// ").unwrap_or(line);
        Self::ROLES
            .iter()
            .find(|(marker, _)| line.starts_with(marker))
            .map(|(_, folder)| *folder)
    }

    fn blocks(source: &str) -> Vec<(usize, &'static str)> {
        let mut offset = 0;
        let mut blocks = Vec::new();
        let mut imports = None;
        for line in source.split_inclusive('\n') {
            if line.starts_with("import ") {
                imports.get_or_insert(offset);
            } else if let Some(folder) = Self::folder(line) {
                blocks.push((imports.unwrap_or(offset), folder));
                imports = None;
            } else if !line.starts_with("#[") {
                imports = None;
            }
            offset += line.len();
        }
        blocks
    }

    fn class_name(block: &str) -> &str {
        for line in block.lines() {
            let declaration = line
                .strip_prefix("class ")
                .or_else(|| line.strip_prefix("struct "))
                .or_else(|| line.strip_prefix("type "));
            if let Some(declaration) = declaration {
                return declaration
                    .split(|character: char| !character.is_ascii_alphanumeric())
                    .next()
                    .expect("class name");
            }
        }
        panic!("fixture block declares a class");
    }

    fn write(root: &Path, extension: &str, contents: &str) -> String {
        let source = contents.trim_end().trim_end_matches("?>");
        let source = source.trim_start().strip_prefix("<?php").unwrap_or(source);
        let blocks = Self::blocks(source);
        let mut controller = String::new();
        for (index, &(start, folder)) in blocks.iter().enumerate() {
            let end = blocks.get(index + 1).map_or(source.len(), |next| next.0);
            let block = &source[start..end];
            let relative = format!("server/{folder}/{}.{extension}", Self::class_name(block));
            let text = if extension == "php" {
                format!("<?php\n{block}")
            } else {
                block.to_owned()
            };
            let path = root.join(&relative);
            fs::create_dir_all(path.parent().expect("parent")).expect("directories");
            fs::write(path, text).expect("source");
            if folder == "controllers" {
                controller = relative;
            }
        }
        assert!(!controller.is_empty(), "the fixture declares a controller");
        controller
    }
}

fn available(program: &str) -> bool {
    let flag = match program {
        "kotlinc" => "-version",
        "go" => "version",
        _ => "--version",
    };
    let found = crate::support::program_probe::ProgramProbe::answers(program, flag);
    crate::support::toolchain::Toolchain::checked(program, found)
}

fn source(root: &tempfile::TempDir, extension: &str, contents: &str) -> HandlerSource {
    let relative = Layout::write(root.path(), extension, contents);
    HandlerSource::discover(root.path(), &relative).expect("annotated source prepares")
}

fn supervisor() -> HandlerSupervisor {
    HandlerSupervisor::new(HandlerSupervisorOptions {
        default_timeout: Duration::from_mins(2),
        ..HandlerSupervisorOptions::default()
    })
    .expect("valid supervisor")
}

async fn assert_controller(extension: &str, program: &str, contents: &str) {
    if !available(program) {
        eprintln!("Skipping {extension} annotated runtime: {program} unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = source(&root, extension, contents);
    let supervisor = supervisor();
    assert_provider_scopes(&source, &supervisor).await;
    assert_zero_argument_response(&source, &supervisor).await;
    let missing = supervisor
        .invoke(
            &source,
            &HandlerRequest::route("missing", "/products", HttpMethod::Delete),
            &HandlerCancellation::default(),
        )
        .await
        .expect("missing method is normal response");
    assert_eq!(missing.status, 405);
}

async fn assert_provider_scopes(source: &HandlerSource, supervisor: &HandlerSupervisor) {
    for attempt in 0..2 {
        let response = supervisor
            .invoke(
                source,
                &HandlerRequest::route(format!("scopes_{attempt}"), "/products", HttpMethod::Get),
                &HandlerCancellation::default(),
            )
            .await
            .expect("instance controller responds");
        assert_eq!(response.status, 200);
        let json: serde_json::Value =
            serde_json::from_str(&response.body.expect("response body").data).expect("JSON body");
        assert_eq!(
            json,
            serde_json::json!({"singleton": true, "request": true, "transient": true, "method": "GET"})
        );
    }
}

async fn assert_zero_argument_response(source: &HandlerSource, supervisor: &HandlerSupervisor) {
    let response = supervisor
        .invoke(
            source,
            &HandlerRequest::route("no_arguments", "/products", HttpMethod::Post),
            &HandlerCancellation::default(),
        )
        .await
        .expect("zero-argument endpoint responds");
    assert_eq!(response.status, 202);
    assert_eq!(
        response.body.expect("created body").data,
        "{\"created\":true}"
    );
    assert_eq!(
        response.headers.get("x-controller"),
        Some(&vec![String::from("products")])
    );
}

struct Fixtures;

impl Fixtures {
    const JAVA: &str = r#"
@Service
class SharedService {}
@Service
@Lifetime("request")
class PerRequestService {}
@Service
@Lifetime("transient")
class FreshService {}
@Service
@Lifetime("request")
@Needs({SharedService.class, PerRequestService.class, FreshService.class})
class LeftService {
    final SharedService shared; final PerRequestService request; final FreshService fresh;
    LeftService(SharedService shared, PerRequestService request, FreshService fresh) {
        this.shared = shared; this.request = request; this.fresh = fresh;
    }
}
@Service
@Lifetime("request")
@Needs({SharedService.class, PerRequestService.class, FreshService.class})
class RightService {
    final SharedService shared; final PerRequestService request; final FreshService fresh;
    RightService(SharedService shared, PerRequestService request, FreshService fresh) {
        this.shared = shared; this.request = request; this.fresh = fresh;
    }
}
@RequestRoute("/products")
@Public
@Needs({LeftService.class, RightService.class})
class ProductsController {
    final LeftService left; final RightService right;
    ProductsController(LeftService left, RightService right) { this.left = left; this.right = right; }
    @GetRoute
    Object getProducts(com.sun.net.httpserver.HttpExchange exchange) {
        return java.util.Map.of(
            "singleton", left.shared == right.shared,
            "request", left.request == right.request,
            "transient", left.fresh != right.fresh,
            "method", exchange.getRequestMethod());
    }
    @PostRoute
    void postProduct(com.sun.net.httpserver.HttpExchange exchange) throws java.io.IOException {
        byte[] body = "{\"created\":true}".getBytes(java.nio.charset.StandardCharsets.UTF_8);
        exchange.getResponseHeaders().add("content-type", "application/json");
        exchange.getResponseHeaders().add("x-controller", "products");
        exchange.sendResponseHeaders(202, body.length);
        exchange.getResponseBody().write(body);
    }
}
"#;

    const KOTLIN: &str = r#"
@Service
class SharedService
@Service
@Lifetime("request")
class PerRequestService
@Service
@Lifetime("transient")
class FreshService
@Service
@Lifetime("request")
@Needs(SharedService::class, PerRequestService::class, FreshService::class)
class LeftService(val shared: SharedService, val request: PerRequestService, val fresh: FreshService)
@Service
@Lifetime("request")
@Needs(SharedService::class, PerRequestService::class, FreshService::class)
class RightService(val shared: SharedService, val request: PerRequestService, val fresh: FreshService)
@RequestRoute("/products")
@Public
@Needs(LeftService::class, RightService::class)
class ProductsController(val left: LeftService, val right: RightService) {
    @GetRoute
    fun getProducts(exchange: com.sun.net.httpserver.HttpExchange): Any? = mapOf(
        "singleton" to (left.shared === right.shared),
        "request" to (left.request === right.request),
        "transient" to (left.fresh !== right.fresh),
        "method" to exchange.requestMethod)
    @PostRoute
    fun postProduct(exchange: com.sun.net.httpserver.HttpExchange) {
        val body = "{\"created\":true}".toByteArray()
        exchange.responseHeaders.add("content-type", "application/json")
        exchange.responseHeaders.add("x-controller", "products")
        exchange.sendResponseHeaders(202, body.size.toLong())
        exchange.responseBody.write(body)
    }
}
"#;

    const CSHARP: &str = r#"
[Service]
class SharedService {}
[Service]
[Lifetime("request")]
class PerRequestService {}
[Service]
[Lifetime("transient")]
class FreshService {}
[Service]
[Lifetime("request")]
[Needs(typeof(SharedService), typeof(PerRequestService), typeof(FreshService))]
class LeftService {
    internal SharedService shared; internal PerRequestService request; internal FreshService fresh;
    public LeftService(SharedService shared, PerRequestService request, FreshService fresh) {
        this.shared = shared; this.request = request; this.fresh = fresh;
    }
}
[Service]
[Lifetime("request")]
[Needs(typeof(SharedService), typeof(PerRequestService), typeof(FreshService))]
class RightService {
    internal SharedService shared; internal PerRequestService request; internal FreshService fresh;
    public RightService(SharedService shared, PerRequestService request, FreshService fresh) {
        this.shared = shared; this.request = request; this.fresh = fresh;
    }
}
[RequestRoute("/products")]
[Public]
[Needs(typeof(LeftService), typeof(RightService))]
class ProductsController {
    readonly LeftService left; readonly RightService right;
    public ProductsController(LeftService left, RightService right) { this.left = left; this.right = right; }
    [GetRoute]
    public object GetProducts(HttpRequestMessage request) {
        return new {
            singleton = object.ReferenceEquals(left.shared, right.shared),
            request = object.ReferenceEquals(left.request, right.request),
            transient = !object.ReferenceEquals(left.fresh, right.fresh),
            method = request.Method.Method
        };
    }
    [PostRoute]
    public HttpResponseMessage PostProduct() {
        var response = new HttpResponseMessage(System.Net.HttpStatusCode.Accepted) { Content = new StringContent("{\"created\":true}", System.Text.Encoding.UTF8, "application/json") };
        response.Headers.Add("x-controller", "products");
        return response;
    }
}
"#;

    const RUST: &str = r#"
#[yon::service]
struct SharedService;
impl SharedService { fn new() -> Self { Self } }
#[yon::service]
#[yon::lifetime("request")]
struct PerRequestService;
impl PerRequestService { fn new() -> Self { Self } }
#[yon::service]
#[yon::lifetime("transient")]
struct FreshService;
impl FreshService { fn new() -> Self { Self } }
#[yon::service]
#[yon::lifetime("request")]
#[yon::needs(SharedService, PerRequestService, FreshService)]
struct LeftService { shared: std::sync::Arc<SharedService>, request: std::sync::Arc<PerRequestService>, fresh: std::sync::Arc<FreshService> }
impl LeftService {
    fn new(shared: std::sync::Arc<SharedService>, request: std::sync::Arc<PerRequestService>, fresh: std::sync::Arc<FreshService>) -> Self {
        Self { shared, request, fresh }
    }
}
#[yon::service]
#[yon::lifetime("request")]
#[yon::needs(SharedService, PerRequestService, FreshService)]
struct RightService { shared: std::sync::Arc<SharedService>, request: std::sync::Arc<PerRequestService>, fresh: std::sync::Arc<FreshService> }
impl RightService {
    fn new(shared: std::sync::Arc<SharedService>, request: std::sync::Arc<PerRequestService>, fresh: std::sync::Arc<FreshService>) -> Self {
        Self { shared, request, fresh }
    }
}
#[RequestRoute("/products")]
#[Public]
#[yon::needs(LeftService, RightService)]
struct ProductsController { left: std::sync::Arc<LeftService>, right: std::sync::Arc<RightService> }
impl ProductsController {
    fn new(left: std::sync::Arc<LeftService>, right: std::sync::Arc<RightService>) -> Self { Self { left, right } }
    #[GetRoute]
    fn get_products(&self, request: &http::Request<Vec<u8>>) -> http::Response<Vec<u8>> {
        let body = format!(
            "{{\"singleton\":{},\"request\":{},\"transient\":{},\"method\":\"{}\"}}",
            std::sync::Arc::ptr_eq(&self.left.shared, &self.right.shared),
            std::sync::Arc::ptr_eq(&self.left.request, &self.right.request),
            !std::sync::Arc::ptr_eq(&self.left.fresh, &self.right.fresh),
            request.method());
        http::Response::builder()
            .header("content-type", "application/json")
            .body(body.into_bytes())
            .unwrap_or_default()
    }
    #[PostRoute]
    fn post_product(&self) -> http::Response<Vec<u8>> {
        http::Response::builder()
            .status(202)
            .header("content-type", "application/json")
            .header("x-controller", "products")
            .body(b"{\"created\":true}".to_vec())
            .unwrap_or_default()
    }
}
"#;

    const GO: &str = r#"
// @yon::Service
type SharedService struct {
	serial int
}
// @yon::Service
// @yon::Lifetime("request")
type PerRequestService struct {
	serial int
}
// @yon::Service
// @yon::Lifetime("transient")
type FreshService struct {
	serial int
}
// @yon::Service
// @yon::Lifetime("request")
// @yon::Needs(SharedService, PerRequestService, FreshService)
type LeftService struct {
	shared  *SharedService
	request *PerRequestService
	fresh   *FreshService
}
// @yon::Service
// @yon::Lifetime("request")
// @yon::Needs(SharedService, PerRequestService, FreshService)
type RightService struct {
	shared  *SharedService
	request *PerRequestService
	fresh   *FreshService
}
import "io"
import "net/http"
import "strings"
// @RequestRoute("/products")
// @Public
// @yon::Needs(LeftService, RightService)
type ProductsController struct {
	left  *LeftService
	right *RightService
}

// @GetRoute
func (controller *ProductsController) GetProducts(request *http.Request) any {
	return map[string]any{
		"singleton": controller.left.shared == controller.right.shared,
		"request":   controller.left.request == controller.right.request,
		"transient": controller.left.fresh != controller.right.fresh,
		"method":    request.Method,
	}
}

// @PostRoute
func (controller *ProductsController) PostProduct() *http.Response {
	header := http.Header{"Content-Type": {"application/json"}, "X-Controller": {"products"}}
	body := io.NopCloser(strings.NewReader(`{"created":true}`))
	return &http.Response{StatusCode: http.StatusAccepted, Header: header, Body: body}
}
"#;
}

#[tokio::test]
async fn java_annotations_resolve_scopes_and_preserve_responses() {
    assert_controller("java", "java", Fixtures::JAVA).await;
}

#[tokio::test]
async fn kotlin_annotations_resolve_scopes_and_preserve_responses() {
    assert_controller("kt", "kotlinc", Fixtures::KOTLIN).await;
}

#[tokio::test]
async fn csharp_annotations_resolve_scopes_and_preserve_responses() {
    assert_controller("cs", "dotnet", Fixtures::CSHARP).await;
}

#[tokio::test]
async fn rust_annotations_resolve_scopes_and_preserve_responses() {
    assert_controller("rs", "rustc", Fixtures::RUST).await;
}

#[tokio::test]
async fn go_annotations_resolve_scopes_and_preserve_responses() {
    assert_controller("go", "go", Fixtures::GO).await;
}

struct StreamFixtures;

impl StreamFixtures {
    const GENERATORS: [(&str, &str, &str); 6] = [
        (
            "groovy",
            "groovyc",
            r"
@RequestRoute('/products')
@Public
class EventsController {
    @StreamRoute
    Iterator<Object> streamEvents() {
        return (1..2).collect { value -> [value: value] }.iterator()
    }
}
",
        ),
        (
            "java",
            "java",
            r#"
@RequestRoute("/products")
@Public
class EventsController {
    @StreamRoute
    java.util.stream.Stream<Object> streamEvents() {
        return java.util.stream.Stream.of(java.util.Map.of("value", 1), java.util.Map.of("value", 2));
    }
}
"#,
        ),
        (
            "java",
            "java",
            r#"
@RequestRoute("/products")
@Public
class IteratorEventsController {
    @StreamRoute
    java.util.Iterator<Object> streamEvents() {
        return java.util.List.<Object>of(java.util.Map.of("value", 1), java.util.Map.of("value", 2)).iterator();
    }
}
"#,
        ),
        (
            "kt",
            "kotlinc",
            r#"
@RequestRoute("/products")
@Public
class EventsController {
    @StreamRoute
    fun streamEvents() = sequence {
        yield(mapOf("value" to 1))
        yield(mapOf("value" to 2))
    }
}
"#,
        ),
        (
            "cs",
            "dotnet",
            r#"
[RequestRoute("/products")]
[Public]
class EventsController {
    [StreamRoute]
    public System.Collections.Generic.IEnumerable<object> StreamEvents() {
        yield return new { value = 1 };
        yield return new { value = 2 };
    }
}
"#,
        ),
        (
            "cs",
            "dotnet",
            r#"
[RequestRoute("/products")]
[Public]
class AsyncEventsController {
    [StreamRoute]
    public async System.Collections.Generic.IAsyncEnumerable<object> StreamEvents() {
        await System.Threading.Tasks.Task.Yield();
        yield return new { value = 1 };
        yield return new { value = 2 };
    }
}
"#,
        ),
    ];
}

#[tokio::test]
async fn annotated_generators_keep_stream_discovery_and_delivery() {
    for (extension, program, contents) in StreamFixtures::GENERATORS {
        if !available(program) {
            eprintln!("Skipping {extension} annotated streaming: {program} unavailable");
            continue;
        }
        let root = tempfile::tempdir().expect("project");
        let source = source(&root, extension, contents);
        assert!(source.streams_method(HttpMethod::Get));
        let (sender, mut receiver) = tokio::sync::mpsc::channel(4);
        supervisor()
            .invoke_streaming(
                &source,
                &HandlerRequest::route("events", "/products", HttpMethod::Get),
                sender,
            )
            .await
            .expect("annotated generator completes");
        let first = receiver.recv().await.expect("first frame");
        let second = receiver.recv().await.expect("second frame");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&first.data).expect("first JSON event"),
            serde_json::json!({"value":1})
        );
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&second.data).expect("second JSON event"),
            serde_json::json!({"value":2})
        );
        assert!(receiver.recv().await.is_none());
    }
}
