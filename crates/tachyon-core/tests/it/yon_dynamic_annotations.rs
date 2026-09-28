#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use tachyon_contracts::{HandlerRequest, HttpMethod};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
};

struct Layout;

impl Layout {
    const ROLES: [(&'static str, &'static str); 14] = [
        ("@RequestRoute", "controllers"),
        ("@request_route", "controllers"),
        ("#[RequestRoute", "controllers"),
        ("[RequestRoute", "controllers"),
        ("@Service", "services"),
        ("@service", "services"),
        ("#[Service", "services"),
        ("[Service", "services"),
        ("#[yon::service", "services"),
        ("@Store", "stores"),
        ("@store", "stores"),
        ("#[Store", "stores"),
        ("[Store", "stores"),
        ("#[yon::store", "stores"),
    ];

    fn folder(line: &str) -> Option<&'static str> {
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

fn available(extension: &str) -> bool {
    let program = match extension {
        "py" => "python3",
        "php" => "php-cgi",
        "ts" => "bun",
        _ => "node",
    };
    let available = Command::new(program)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    if !available {
        eprintln!("Skipping .{extension} annotated Yon runtime check: {program} is unavailable.");
    }
    available
}

fn source(root: &tempfile::TempDir, extension: &str, contents: &str) -> HandlerSource {
    let relative = Layout::write(root.path(), extension, contents);
    HandlerSource::discover(root.path(), &relative).expect("annotated source discovery")
}

fn supervisor(extension: &str) -> HandlerSupervisor {
    let mut options = HandlerSupervisorOptions::default();
    if extension == "ts" {
        options.runtimes.javascript = "bun".into();
    }
    HandlerSupervisor::new(options).expect("supervisor")
}

struct Fixtures;

impl Fixtures {
    const JAVASCRIPT: &str = r#"
@Store()
class InventoryStore {
  static created = 0;
  constructor() { this.id = ++InventoryStore.created; }
}
@Service()
@Needs(InventoryStore)
class LeftService {
  constructor(inventory) { this.inventory = inventory; }
}
@Service()
@Needs(InventoryStore)
class RightService {
  constructor(inventory) { this.inventory = inventory; }
}
@RequestRoute("/products")
@Public
@Needs(LeftService, RightService)
class ProductsController {
  constructor(left, right) { this.left = left; this.right = right; }
  @GetRoute
  async getProducts(request) {
    return {same: this.left.inventory === this.right.inventory, count: InventoryStore.created, method: request.method};
  }
  @PostRoute
  postProduct() {
    return new Response('<if true>{{ untouched }}</if>', {status: 202, headers: {'content-type': 'text/html'}});
  }
}
"#;

    const TYPESCRIPT: &str = r#"
@Store()
class InventoryStore {
  static created = 0;
  id: number;
  constructor() { this.id = ++InventoryStore.created; }
}
@Service()
@Needs(InventoryStore)
class LeftService {
  inventory: InventoryStore;
  constructor(inventory: InventoryStore) { this.inventory = inventory; }
}
@Service()
@Needs(InventoryStore)
class RightService {
  inventory: InventoryStore;
  constructor(inventory: InventoryStore) { this.inventory = inventory; }
}
@RequestRoute("/products")
@Public
@Needs(LeftService, RightService)
class ProductsController {
  left: LeftService;
  right: RightService;
  constructor(left: LeftService, right: RightService) { this.left = left; this.right = right; }
  @GetRoute
  async getProducts(request: Request) {
    return {same: this.left.inventory === this.right.inventory, count: InventoryStore.created, method: request.method};
  }
  @PostRoute
  postProduct() {
    return new Response('<if true>{{ untouched }}</if>', {status: 202, headers: {'content-type': 'text/html'}});
  }
}
"#;

    const PYTHON: &str = r#"
@store()
class InventoryStore:
    created = 0
    def __init__(self):
        InventoryStore.created += 1
        self.id = InventoryStore.created

@service()
@needs(InventoryStore)
class LeftService:
    def __init__(self, inventory):
        self.inventory = inventory

@service()
@needs(InventoryStore)
class RightService:
    def __init__(self, inventory):
        self.inventory = inventory

import http
@request_route("/products")
@public
@needs(LeftService, RightService)
class ProductsController:
    def __init__(self, left, right):
        self.left = left
        self.right = right
    @get_route
    async def get_products(self, request):
        return {"same": self.left.inventory is self.right.inventory, "count": InventoryStore.created, "method": request.get_method()}
    @post_route
    def post_product(self):
        return (http.HTTPStatus.ACCEPTED, {"content-type": "text/html"}, "<if true>{{ untouched }}</if>")
"#;

    const PHP: &str = r#"<?php
#[Store]
class InventoryStore {
    public static int $created = 0;
    public int $id;
    public function __construct() { $this->id = ++self::$created; }
}
#[Service]
#[Needs(InventoryStore::class)]
class LeftService {
    public function __construct(public InventoryStore $inventory) {}
}
#[Service]
#[Needs(InventoryStore::class)]
class RightService {
    public function __construct(public InventoryStore $inventory) {}
}
#[RequestRoute("/products")]
#[Public]
#[Needs(LeftService::class, RightService::class)]
class ProductsController {
    public function __construct(public LeftService $left, public RightService $right) {}
    #[GetRoute]
    public function getProducts() {
        return ['same' => $this->left->inventory === $this->right->inventory, 'count' => InventoryStore::$created, 'method' => $_SERVER['REQUEST_METHOD']];
    }
    #[PostRoute]
    public function postProduct() {
        http_response_code(202);
        header('Content-Type: text/html');
        echo '<if true>{{ untouched }}</if>';
    }
}
?>
"#;
}

fn fixture(extension: &str, transient: bool) -> String {
    let text = match extension {
        "ts" => Fixtures::TYPESCRIPT,
        "py" => Fixtures::PYTHON,
        "php" => Fixtures::PHP,
        _ => Fixtures::JAVASCRIPT,
    };
    if !transient {
        return text.to_owned();
    }
    text.replace("@Store()", "@Store()\n@Lifetime('transient')")
        .replace("@store()", "@store()\n@lifetime('transient')")
        .replace("#[Store]", "#[Store]\n#[Lifetime('transient')]")
}

#[tokio::test]
async fn annotated_controllers_resolve_diamonds_transients_and_request_isolation() {
    for extension in ["js", "ts", "py", "php"] {
        if !available(extension) {
            continue;
        }
        for transient in [false, true] {
            assert_scope_identity(extension, transient).await;
        }
    }
}

async fn assert_scope_identity(extension: &str, transient: bool) {
    let root = tempfile::tempdir().expect("project");
    let source = source(&root, extension, &fixture(extension, transient));
    let supervisor = supervisor(extension);
    for iteration in 0..2 {
        let request =
            HandlerRequest::route(format!("request_{iteration}"), "/products", HttpMethod::Get);
        let response = supervisor
            .invoke(&source, &request, &HandlerCancellation::default())
            .await
            .expect("response");
        assert!(
            response.error.is_none(),
            "{extension}: {:?}",
            response.error
        );
        let value: serde_json::Value =
            serde_json::from_str(&response.body.expect("body").data).expect("JSON");
        assert_eq!(
            value,
            serde_json::json!({"same": !transient, "count": if transient {2} else {1}, "method": "GET"}),
            "{extension}"
        );
    }
}

#[tokio::test]
async fn annotated_controllers_preserve_explicit_opaque_html() {
    for extension in ["js", "ts", "py", "php"] {
        if !available(extension) {
            continue;
        }
        let root = tempfile::tempdir().expect("project");
        let source = source(&root, extension, &fixture(extension, false));
        let request = HandlerRequest::route("html", "/products", HttpMethod::Post);
        let response = supervisor(extension)
            .invoke(&source, &request, &HandlerCancellation::default())
            .await
            .expect("response");
        assert!(
            response.error.is_none(),
            "{extension}: {:?}",
            response.error
        );
        assert_eq!(response.status, 202, "{extension}");
        let content_type = if extension == "php" {
            "text/html;charset=UTF-8"
        } else {
            "text/html"
        };
        assert_eq!(
            response.headers["content-type"],
            [content_type],
            "{extension}"
        );
        assert_eq!(
            response.body.expect("HTML body").data,
            "<if true>{{ untouched }}</if>",
            "{extension}"
        );
    }
}

#[tokio::test]
async fn route_annotations_dispatch_every_http_verb() {
    if !available("js") {
        return;
    }
    let methods = [
        ("GET", "GetRoute", "getItems", HttpMethod::Get),
        ("POST", "PostRoute", "postItems", HttpMethod::Post),
        ("PUT", "PutRoute", "putItems", HttpMethod::Put),
        ("PATCH", "PatchRoute", "patchItems", HttpMethod::Patch),
        ("DELETE", "DeleteRoute", "deleteItems", HttpMethod::Delete),
    ];
    let members = methods
        .iter()
        .map(|(_, marker, name, _)| format!("  @{marker}\n  {name}(request) {{ return new Response('ok', {{status: 200, headers: {{'x-controller-method': '{name}', 'x-request-method': request.method}}}}); }}"))
        .collect::<Vec<_>>()
        .join("\n");
    let contents = format!(
        "@RequestRoute(\"/products\")\n@Public\nclass MethodsController {{\n{members}\n  *helper() {{ yield 'not an endpoint'; }}\n}}\n"
    );
    let root = tempfile::tempdir().expect("project");
    let source = source(&root, "js", &contents);
    let supervisor = supervisor("js");
    for (verb, _, name, method) in methods {
        assert!(
            !source.streams_method(method),
            "helper cannot mark {verb} as streaming"
        );
        let request = HandlerRequest::route(format!("dispatch_{verb}"), "/products", method);
        let response = supervisor
            .invoke(&source, &request, &HandlerCancellation::default())
            .await
            .expect("route method response");
        assert!(response.error.is_none(), "{verb}: {:?}", response.error);
        assert_eq!(response.status, 200, "{verb}");
        assert_eq!(response.headers["x-controller-method"], [name]);
        assert_eq!(response.headers["x-request-method"], [verb]);
    }
}

#[tokio::test]
async fn annotated_generators_deliver_stream_events() {
    let javascript = "@RequestRoute(\"/products\")\n@Public\nclass ProductsController {\n @StreamRoute\n async *streamProducts() { yield 'first'; yield 'second'; }\n}\n";
    let python = "@request_route(\"/products\")\n@public\nclass ProductsController:\n    @stream_route\n    def stream_products(self):\n        yield 'first'\n        yield 'second'\n";
    let php = "<?php\n#[RequestRoute(\"/products\")]\n#[Public]\nclass ProductsController {\n #[StreamRoute]\n public function streamProducts() { yield 'first'; yield 'second'; }\n}\n";
    for (extension, contents) in [
        ("js", javascript),
        ("ts", javascript),
        ("py", python),
        ("php", php),
    ] {
        if !available(extension) {
            continue;
        }
        let root = tempfile::tempdir().expect("project");
        let source = source(&root, extension, contents);
        assert!(source.streams_method(HttpMethod::Get), "{extension}");
        let request = HandlerRequest::route("events", "/products", HttpMethod::Get);
        let (sender, mut receiver) = tokio::sync::mpsc::channel(4);
        let result = supervisor(extension)
            .invoke_streaming(&source, &request, sender)
            .await;
        assert!(result.is_ok(), "{extension}: {result:?}");
        let mut events = Vec::new();
        while let Some(event) = receiver.recv().await {
            events.push(event);
        }
        assert_eq!(events.len(), 2, "{extension}: {events:?}");
        for (event, expected) in events.iter().zip(["first", "second"]) {
            let value: String = serde_json::from_str(&event.data).expect("JSON event");
            assert_eq!(value, expected, "{extension}");
        }
    }
}
