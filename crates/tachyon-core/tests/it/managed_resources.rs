#![allow(clippy::expect_used)]

use crate::support::yon_layout;

use tachyon_contracts::{HandlerRequest, HttpMethod};
use tachyon_core::{
    ApplicationConfig, HandlerCancellation, HandlerSource, HandlerSupervisor,
    HandlerSupervisorOptions,
};

async fn round_trip(extension: &str, source: &str) {
    if !runtime_available(extension) {
        return;
    }
    let root = resource_project();
    let entry = yon_layout::write_fixture(
        root.path(),
        &format!("server/controllers/ProductController.{extension}"),
        source,
    );
    assert_round_trip(root.path(), extension, &entry).await;
}

async fn plain_round_trip(extension: &str, source: &str) {
    if !runtime_available(extension) {
        return;
    }
    let root = resource_project();
    let entry = format!("server/workers/products.{extension}");
    std::fs::create_dir_all(root.path().join("server/workers")).expect("workers");
    std::fs::write(root.path().join(&entry), source).expect("handler");
    assert_round_trip(root.path(), extension, &entry).await;
}

fn resource_project() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("project");
    std::fs::create_dir_all(root.path().join("server")).expect("server");
    std::fs::write(
        root.path().join("server/application.json"),
        r#"{"datasource":{"driver":"sqlite","url":"sqlite::memory:"}}"#,
    )
    .expect("configuration");
    root
}

async fn assert_round_trip(root: &std::path::Path, extension: &str, entry: &str) {
    let source = HandlerSource::discover(root, entry).expect("resource field inference");
    let mut options = HandlerSupervisorOptions {
        application: ApplicationConfig::load(root).expect("configuration"),
        ..HandlerSupervisorOptions::default()
    };
    if extension == "ts" {
        options.runtimes.javascript = "bun".into();
    }
    let supervisor = HandlerSupervisor::new(options).expect("supervisor");
    assert_stored_values(extension, &source, &supervisor).await;
}

fn runtime_available(extension: &str) -> bool {
    let program = match extension {
        "java" => "java",
        "kt" => "kotlinc",
        "cs" => "dotnet",
        "php" => "php-cgi",
        "py" => "python3",
        "rb" => "ruby",
        "rs" => "rustc",
        "go" => "go",
        "dart" => "dart",
        _ => "node",
    };
    let paths = std::env::var_os("PATH").unwrap_or_default();
    let available = std::env::split_paths(&paths).any(|directory| {
        directory.join(program).is_file()
            || (cfg!(windows)
                && ["exe", "bat", "cmd"]
                    .iter()
                    .any(|suffix| directory.join(format!("{program}.{suffix}")).is_file()))
    });
    if !available {
        assert_ne!(
            std::env::var("TAC_STORAGE_REQUIRE_TOOLCHAINS").as_deref(),
            Ok("1"),
            "Required resource runtime {program} is unavailable"
        );
        eprintln!("Skipping .{extension} managed resource execution: {program} unavailable");
        return false;
    }
    true
}

async fn assert_stored_values(
    extension: &str,
    source: &HandlerSource,
    supervisor: &HandlerSupervisor,
) {
    for (index, method) in [HttpMethod::Post, HttpMethod::Get, HttpMethod::Get]
        .into_iter()
        .enumerate()
    {
        let request =
            HandlerRequest::route(format!("storage_{extension}_{index}"), "/products", method);
        let response = supervisor
            .invoke(source, &request, &HandlerCancellation::default())
            .await
            .expect("storage invocation");
        assert_eq!(response.status, 200, "{extension}: {:?}", response.error);
        let json: serde_json::Value =
            serde_json::from_str(&response.body.expect("body").data).expect("JSON");
        if index > 0 {
            assert_eq!(
                json,
                serde_json::json!([{"name":"quoted'; SELECT secret; --"}]),
                "{extension}"
            );
        }
    }
}

#[tokio::test]
async fn javascript_resources_are_inferred_and_shared() {
    round_trip("js", r#"
@Service()
@NeedsAll()
class ProductService {
  dataSource;
  async initialize() {
    await this.dataSource.execute("CREATE TABLE products (id INTEGER PRIMARY KEY, name TEXT)");
    return this.dataSource.execute("INSERT INTO products (id, name) VALUES (?, ?)", [1, "quoted'; SELECT secret; --"]);
  }
  find() { return this.dataSource.query("SELECT name FROM products WHERE id = ?", [1]); }
}
@RequestRoute("/products")
@Public
@NeedsAll()
class ProductController {
  productService;
  @PostRoute
  postProducts() { return this.productService.initialize(); }
  @GetRoute
  getProducts() { return this.productService.find(); }
}
"#).await;
}

#[tokio::test]
async fn typescript_resources_are_inferred_and_shared() {
    round_trip("ts", r#"
@Service()
@NeedsAll()
class ProductService {
  dataSource: DataSource;
  async initialize() {
    await this.dataSource.execute("CREATE TABLE products (id INTEGER PRIMARY KEY, name TEXT)", []);
    return this.dataSource.execute("INSERT INTO products (id, name) VALUES (?, ?)", [1, "quoted'; SELECT secret; --"]);
  }
  find() { return this.dataSource.query("SELECT name FROM products WHERE id = ?", [1]); }
}
@RequestRoute("/products")
@Public
@NeedsAll()
class ProductController {
  productService: ProductService;
  @PostRoute
  postProducts() { return this.productService.initialize(); }
  @GetRoute
  getProducts() { return this.productService.find(); }
}
"#).await;
}

#[tokio::test]
async fn python_resources_are_inferred_and_shared() {
    round_trip("py", r#"
@service()
@needs_all()
class ProductService:
    data_source: DataSource
    def initialize(self):
        self.data_source.execute("CREATE TABLE products (id INTEGER PRIMARY KEY, name TEXT)")
        return self.data_source.execute("INSERT INTO products (id, name) VALUES (?, ?)", [1, "quoted'; SELECT secret; --"])
    def find(self):
        return self.data_source.query("SELECT name FROM products WHERE id = ?", [1])
@request_route("/products")
@public
@needs_all()
class ProductController:
    products: ProductService
    @post_route
    def post_products(self):
        return self.products.initialize()
    @get_route
    def get_products(self):
        return self.products.find()
"#).await;
}

#[tokio::test]
async fn rust_resources_are_inferred_and_shared() {
    round_trip("rs", r#"
#[Service()]
#[NeedsAll()]
struct ProductService { data_source: std::sync::Arc<DataSource> }
impl ProductService {
    fn initialize(&self) -> Result<Json, YonResourceError> {
        self.data_source.execute("CREATE TABLE products (id INTEGER PRIMARY KEY, name TEXT)", vec![])?;
        self.data_source.execute("INSERT INTO products (id, name) VALUES (?, ?)", vec![1.into(), "quoted'; SELECT secret; --".into()])
    }
    fn find(&self) -> Result<Json, YonResourceError> {
        self.data_source.query("SELECT name FROM products WHERE id = ?", vec![1.into()])
    }
}
#[RequestRoute("/products")]
#[Public]
#[NeedsAll()]
struct ProductController { products: std::sync::Arc<ProductService> }
impl ProductController {
    #[PostRoute]
    fn post_products(&self) -> Result<Json, YonResourceError> { self.products.initialize() }
    #[GetRoute]
    fn get_products(&self) -> Result<Json, YonResourceError> { self.products.find() }
}
"#).await;
}

#[tokio::test]
async fn java_resources_are_inferred_and_shared() {
    round_trip("java", r#"
@Service()
@NeedsAll()
class ProductService {
    DataSource dataSource;
    Object initialize() {
        dataSource.execute("CREATE TABLE products (id INTEGER PRIMARY KEY, name TEXT)", java.util.List.of());
        return dataSource.execute("INSERT INTO products (id, name) VALUES (?, ?)", java.util.List.of(1, "quoted'; SELECT secret; --"));
    }
    Object find() { return dataSource.query("SELECT name FROM products WHERE id = ?", java.util.List.of(1)); }
}
@RequestRoute("/products")
@Public
@NeedsAll()
class ProductController {
    ProductService products;
    @PostRoute
    Object postProducts() { return products.initialize(); }
    @GetRoute
    Object getProducts() { return products.find(); }
}
"#).await;
}

#[tokio::test]
async fn kotlin_resources_are_inferred_and_shared() {
    round_trip("kt", r#"
@Service()
@NeedsAll()
class ProductService {
    val dataSource: DataSource
    fun initialize(): Any? {
        dataSource.execute("CREATE TABLE products (id INTEGER PRIMARY KEY, name TEXT)", emptyList())
        return dataSource.execute("INSERT INTO products (id, name) VALUES (?, ?)", listOf(1, "quoted'; SELECT secret; --"))
    }
    fun find(): Any? = dataSource.query("SELECT name FROM products WHERE id = ?", listOf(1))
}
@RequestRoute("/products")
@Public
@NeedsAll()
class ProductController {
    val products: ProductService
    @PostRoute
    fun postProducts(): Any? = products.initialize()
    @GetRoute
    fun getProducts(): Any? = products.find()
}
"#).await;
}

#[tokio::test]
async fn csharp_resources_are_inferred_and_shared() {
    round_trip("cs", r#"
[Service()]
[NeedsAll()]
class ProductService {
    DataSource _dataSource;
    public object initialize() {
        _dataSource.Execute("CREATE TABLE products (id INTEGER PRIMARY KEY, name TEXT)", new object[] {});
        return _dataSource.Execute("INSERT INTO products (id, name) VALUES (?, ?)", new object[] {1, "quoted'; SELECT secret; --"});
    }
    public object find() { return _dataSource.Query("SELECT name FROM products WHERE id = ?", new object[] {1}); }
}
[RequestRoute("/products")]
[Public]
[NeedsAll()]
class ProductController {
    ProductService _products;
    [PostRoute]
    public object PostProducts() { return _products.initialize(); }
    [GetRoute]
    public object GetProducts() { return _products.find(); }
}
"#).await;
}

#[tokio::test]
async fn php_resources_are_inferred_and_shared() {
    round_trip("php", r#"<?php
#[Service()]
#[NeedsAll()]
class ProductService {
    private DataSource $dataSource;
    public function initialize(): mixed {
        $this->dataSource->execute("CREATE TABLE products (id INTEGER PRIMARY KEY, name TEXT)", []);
        return $this->dataSource->execute("INSERT INTO products (id, name) VALUES (?, ?)", [1, "quoted'; SELECT secret; --"]);
    }
    public function find(): mixed { return $this->dataSource->query("SELECT name FROM products WHERE id = ?", [1]); }
}
#[RequestRoute("/products")]
#[Public]
#[NeedsAll()]
class ProductController {
    private ProductService $products;
    #[PostRoute]
    public function postProducts(): mixed { return $this->products->initialize(); }
    #[GetRoute]
    public function getProducts(): mixed { return $this->products->find(); }
}
"#).await;
}

#[tokio::test]
async fn go_handlers_reach_the_managed_data_source() {
    plain_round_trip("go", r#"
type Handler struct{}

func (Handler) POST() (any, error) {
	dataSource := DataSource{}
	_, err := dataSource.Execute("CREATE TABLE products (id INTEGER PRIMARY KEY, name TEXT)", nil)
	if err != nil {
		return nil, err
	}
	return dataSource.Execute("INSERT INTO products (id, name) VALUES (?, ?)", []any{1, "quoted'; SELECT secret; --"})
}

func (Handler) GET() (any, error) {
	return DataSource{}.Query("SELECT name FROM products WHERE id = ?", []any{1})
}
"#).await;
}

#[tokio::test]
async fn ruby_handlers_reach_the_managed_data_source() {
    plain_round_trip("rb", r#"
class Handler
  def self.POST(request)
    data_source = DataSource.new
    data_source.execute("CREATE TABLE products (id INTEGER PRIMARY KEY, name TEXT)")
    data_source.execute("INSERT INTO products (id, name) VALUES (?, ?)", [1, "quoted'; SELECT secret; --"])
  end

  def self.GET(request)
    DataSource.new.query("SELECT name FROM products WHERE id = ?", [1])
  end
end
"#).await;
}

#[cfg(unix)]
struct SlowStore;

#[cfg(unix)]
impl SlowStore {
    const PAUSE: std::time::Duration = std::time::Duration::from_secs(6);
    const SHORTEST_ANSWER: std::time::Duration = std::time::Duration::from_millis(5_500);

    async fn controller(extension: &str, source: &str) {
        if !runtime_available(extension) || !runtime_available_program("fylo") {
            return;
        }
        let root = tempfile::tempdir().expect("project");
        let relative = format!("server/controllers/ProductController.{extension}");
        let entry = yon_layout::write_fixture(root.path(), &relative, source);
        Self::answers(extension, root.path(), &entry).await;
    }

    async fn worker(extension: &str, source: &str) {
        if !runtime_available(extension) || !runtime_available_program("fylo") {
            return;
        }
        let root = tempfile::tempdir().expect("project");
        let entry = format!("server/workers/products.{extension}");
        std::fs::create_dir_all(root.path().join("server/workers")).expect("workers");
        std::fs::write(root.path().join(&entry), source).expect("handler");
        Self::answers(extension, root.path(), &entry).await;
    }

    async fn answers(extension: &str, root: &std::path::Path, entry: &str) {
        let fylo_root = Self::fylo_root(root);
        let source = HandlerSource::discover(root, entry).expect("handler");
        let supervisor = Self::supervisor(root, extension);
        Self::assert_answer(extension, &supervisor, &source, "slow_store_warm").await;
        let resumed = Self::pause_fylo(&fylo_root);
        let started = std::time::Instant::now();
        Self::assert_answer(extension, &supervisor, &source, "slow_store_paused").await;
        let took = started.elapsed();
        resumed.join().expect("resume thread");
        assert!(
            took >= Self::SHORTEST_ANSWER,
            "{extension}: the store answered after {took:?}"
        );
    }

    fn fylo_root(project: &std::path::Path) -> std::path::PathBuf {
        let fylo_root = project.join("fylo-root");
        std::fs::create_dir_all(&fylo_root).expect("FYLO root");
        let fylo_root = std::fs::canonicalize(fylo_root).expect("canonical FYLO root");
        let url = format!("fylo://{}", fylo_root.display());
        let datasource =
            serde_json::json!({"datasource": {"driver": "fylo", "url": url, "timeout": 10_000}});
        std::fs::create_dir_all(project.join("server")).expect("server");
        std::fs::write(
            project.join("server/application.json"),
            datasource.to_string(),
        )
        .expect("configuration");
        fylo_root
    }

    fn supervisor(root: &std::path::Path, extension: &str) -> HandlerSupervisor {
        let mut options = HandlerSupervisorOptions {
            application: ApplicationConfig::load(root).expect("configuration"),
            ..HandlerSupervisorOptions::default()
        };
        if extension == "ts" {
            options.runtimes.javascript = "bun".into();
        }
        HandlerSupervisor::new(options).expect("supervisor")
    }

    async fn assert_answer(
        extension: &str,
        supervisor: &HandlerSupervisor,
        source: &HandlerSource,
        id: &str,
    ) {
        let request =
            HandlerRequest::route(format!("{id}_{extension}"), "/products", HttpMethod::Get);
        let response = supervisor
            .invoke(source, &request, &HandlerCancellation::default())
            .await
            .expect("store invocation");
        assert_eq!(response.status, 200, "{extension}: {:?}", response.error);
    }

    fn pause_fylo(fylo_root: &std::path::Path) -> std::thread::JoinHandle<()> {
        let pattern = format!("fylo exec --loop --root {}", fylo_root.display());
        let found = std::process::Command::new("pgrep")
            .args(["-f", &pattern])
            .output()
            .expect("pgrep");
        let process_id = String::from_utf8_lossy(&found.stdout).trim().to_owned();
        let stop = std::process::Command::new("kill")
            .args(["-STOP", &process_id])
            .status();
        assert!(
            stop.is_ok_and(|status| status.success()),
            "pause fylo {process_id}"
        );
        std::thread::spawn(move || {
            std::thread::sleep(Self::PAUSE);
            let resumed = std::process::Command::new("kill")
                .args(["-CONT", &process_id])
                .status();
            assert!(resumed.is_ok_and(|status| status.success()), "resume fylo");
        })
    }
}

fn runtime_available_program(program: &str) -> bool {
    let paths = std::env::var_os("PATH").unwrap_or_default();
    let available =
        std::env::split_paths(&paths).any(|directory| directory.join(program).is_file());
    if !available {
        assert_ne!(
            std::env::var("TAC_STORAGE_REQUIRE_TOOLCHAINS").as_deref(),
            Ok("1"),
            "Required program {program} is unavailable"
        );
    }
    available
}

#[cfg(unix)]
#[tokio::test]
async fn a_javascript_store_call_slower_than_five_seconds_answers() {
    SlowStore::controller(
        "js",
        r#"
@Service()
@NeedsAll()
class ProductService {
  dataSource;
  exists() { return this.dataSource.collectionExists("hd-events"); }
}
@RequestRoute("/products")
@Public
@NeedsAll()
class ProductController {
  productService;
  @GetRoute
  getProducts() { return this.productService.exists(); }
}
"#,
    )
    .await;
}

#[cfg(unix)]
#[tokio::test]
async fn a_typescript_store_call_slower_than_five_seconds_answers() {
    SlowStore::controller(
        "ts",
        r#"
@Service()
@NeedsAll()
class ProductService {
  dataSource: DataSource;
  exists() { return this.dataSource.collectionExists("hd-events"); }
}
@RequestRoute("/products")
@Public
@NeedsAll()
class ProductController {
  productService: ProductService;
  @GetRoute
  getProducts() { return this.productService.exists(); }
}
"#,
    )
    .await;
}

#[cfg(unix)]
#[tokio::test]
async fn a_python_store_call_slower_than_five_seconds_answers() {
    SlowStore::controller(
        "py",
        r#"
@service()
@needs_all()
class ProductService:
    data_source: DataSource
    def exists(self):
        return self.data_source.collection_exists("hd-events")
@request_route("/products")
@public
@needs_all()
class ProductController:
    products: ProductService
    @get_route
    def get_products(self):
        return self.products.exists()
"#,
    )
    .await;
}

#[cfg(unix)]
#[tokio::test]
async fn a_rust_store_call_slower_than_five_seconds_answers() {
    SlowStore::controller("rs", r#"
#[Service()]
#[NeedsAll()]
struct ProductService { data_source: std::sync::Arc<DataSource> }
impl ProductService {
    fn exists(&self) -> Result<Json, YonResourceError> { self.data_source.collection_exists("hd-events") }
}
#[RequestRoute("/products")]
#[Public]
#[NeedsAll()]
struct ProductController { products: std::sync::Arc<ProductService> }
impl ProductController {
    #[GetRoute]
    fn get_products(&self) -> Result<Json, YonResourceError> { self.products.exists() }
}
"#).await;
}

#[cfg(unix)]
#[tokio::test]
async fn a_java_store_call_slower_than_five_seconds_answers() {
    SlowStore::controller(
        "java",
        r#"
@Service()
@NeedsAll()
class ProductService {
    DataSource dataSource;
    Object exists() { return dataSource.collectionExists("hd-events"); }
}
@RequestRoute("/products")
@Public
@NeedsAll()
class ProductController {
    ProductService products;
    @GetRoute
    Object getProducts() { return products.exists(); }
}
"#,
    )
    .await;
}

#[cfg(unix)]
#[tokio::test]
async fn a_kotlin_store_call_slower_than_five_seconds_answers() {
    SlowStore::controller(
        "kt",
        r#"
@Service()
@NeedsAll()
class ProductService {
    val dataSource: DataSource
    fun exists(): Any? = dataSource.collectionExists("hd-events")
}
@RequestRoute("/products")
@Public
@NeedsAll()
class ProductController {
    val products: ProductService
    @GetRoute
    fun getProducts(): Any? = products.exists()
}
"#,
    )
    .await;
}

#[cfg(unix)]
#[tokio::test]
async fn a_csharp_store_call_slower_than_five_seconds_answers() {
    SlowStore::controller(
        "cs",
        r#"
[Service()]
[NeedsAll()]
class ProductService {
    DataSource _dataSource;
    public object exists() { return _dataSource.CollectionExists("hd-events"); }
}
[RequestRoute("/products")]
[Public]
[NeedsAll()]
class ProductController {
    ProductService _products;
    [GetRoute]
    public object GetProducts() { return _products.exists(); }
}
"#,
    )
    .await;
}

#[cfg(unix)]
#[tokio::test]
async fn a_php_store_call_slower_than_five_seconds_answers() {
    SlowStore::controller(
        "php",
        r#"<?php
#[Service()]
#[NeedsAll()]
class ProductService {
    private DataSource $dataSource;
    public function exists(): mixed { return $this->dataSource->collectionExists("hd-events"); }
}
#[RequestRoute("/products")]
#[Public]
#[NeedsAll()]
class ProductController {
    private ProductService $products;
    #[GetRoute]
    public function getProducts(): mixed { return $this->products->exists(); }
}
"#,
    )
    .await;
}

#[cfg(unix)]
#[tokio::test]
async fn a_go_store_call_slower_than_five_seconds_answers() {
    SlowStore::worker(
        "go",
        r#"
type Handler struct{}

func (Handler) GET() (any, error) {
	return DataSource{}.CollectionExists("hd-events")
}
"#,
    )
    .await;
}

#[cfg(unix)]
#[tokio::test]
async fn a_ruby_store_call_slower_than_five_seconds_answers() {
    SlowStore::worker(
        "rb",
        r#"
class Handler
  def self.GET(request)
    DataSource.new.collection_exists("hd-events")
  end
end
"#,
    )
    .await;
}

#[cfg(unix)]
#[tokio::test]
async fn a_dart_store_call_slower_than_five_seconds_answers() {
    SlowStore::worker(
        "dart",
        r"
class Handler {
  Future<Object?> GET() async => await DataSource().collectionExists('hd-events');
}
",
    )
    .await;
}

struct ProxyProbe;

impl ProxyProbe {
    const MARKER: &str = "TAC_PROXY_PROBE";
    const VARIABLES: [&str; 6] = [
        "HTTP_PROXY",
        "http_proxy",
        "HTTPS_PROXY",
        "https_proxy",
        "ALL_PROXY",
        "all_proxy",
    ];
    const JAVASCRIPT: &str = r"export class Handler {
  static async GET() {
    const loopbackBypass = [process.env.NO_PROXY, process.env.no_proxy].some((value) => (value ?? '').includes('127.0.0.1'))
    delete process.env.NO_PROXY
    delete process.env.no_proxy
    return { rows: await new DataSource().query('SELECT 42 AS answer', []), loopbackBypass }
  }
}
";
    const GO: &str = r#"import (
	"os"
	"strings"
)

type Handler struct{}

func (Handler) GET() (any, error) {
	loopbackBypass := strings.Contains(os.Getenv("NO_PROXY")+os.Getenv("no_proxy"), "127.0.0.1")
	os.Unsetenv("NO_PROXY")
	os.Unsetenv("no_proxy")
	rows, err := DataSource{}.Query("SELECT 42 AS answer", nil)
	return map[string]any{"rows": rows, "loopbackBypass": loopbackBypass}, err
}
"#;
    const RUBY: &str = r#"class Handler
  def self.GET(request)
    loopback_bypass = [ENV["NO_PROXY"], ENV["no_proxy"]].any? { |value| value.to_s.include?("127.0.0.1") }
    ENV.delete("NO_PROXY")
    ENV.delete("no_proxy")
    { "rows" => DataSource.new.query("SELECT 42 AS answer", []), "loopbackBypass" => loopback_bypass }
  end
end
"#;
    const DART: &str = r"class Handler {
  Future<Object?> GET() async =>
      {'rows': await DataSource().query('SELECT 42 AS answer', []), 'loopbackBypass': true};
}
";

    fn loopback_requests(proxy: &std::net::TcpListener) -> Vec<String> {
        let mut forwarded = Vec::new();
        while let Ok((mut connection, _)) = proxy.accept() {
            let mut first_bytes = [0_u8; 512];
            connection.set_nonblocking(false).expect("blocking read");
            let timeout = Some(std::time::Duration::from_secs(1));
            connection.set_read_timeout(timeout).expect("read timeout");
            let count = std::io::Read::read(&mut connection, &mut first_bytes).unwrap_or(0);
            let request = String::from_utf8_lossy(&first_bytes[..count]).into_owned();
            if request.contains("127.0.0.1") {
                forwarded.push(request);
            }
        }
        forwarded
    }

    fn project(extension: &str, handler: &str) -> (tempfile::TempDir, HandlerSource) {
        let root = tempfile::tempdir().expect("project");
        let entry = format!("server/workers/answer.{extension}");
        for (relative, text) in [
            (
                "server/application.json",
                r#"{"datasource":{"driver":"sqlite","url":"sqlite::memory:"}}"#,
            ),
            (entry.as_str(), handler),
        ] {
            let path = root.path().join(relative);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
            std::fs::write(path, text).expect("file");
        }
        let source = HandlerSource::discover(root.path(), &entry).expect("handler source");
        (root, source)
    }

    async fn answer(extension: &str, runtime: &str, handler: &str) -> serde_json::Value {
        let (root, source) = Self::project(extension, handler);
        let mut options = HandlerSupervisorOptions {
            application: ApplicationConfig::load(root.path()).expect("configuration"),
            environment: tachyon_core::EnvironmentPolicy::from_names(Self::VARIABLES)
                .expect("proxy allowlist"),
            ..HandlerSupervisorOptions::default()
        };
        options.runtimes.javascript = runtime.into();
        let supervisor = HandlerSupervisor::new(options).expect("supervisor");
        let request = HandlerRequest::route("proxy_probe", "/answer", HttpMethod::Get);
        let response = supervisor
            .invoke(&source, &request, &HandlerCancellation::default())
            .await
            .expect("resource call through the proxy probe");
        let Some(body) = response.body.filter(|_| response.status == 200) else {
            return serde_json::json!({"status": response.status, "error": response.error.map(|error| error.message)});
        };
        serde_json::from_str(&body.data).expect("JSON")
    }
}

#[tokio::test]
async fn proxy_probe_child() {
    if std::env::var_os(ProxyProbe::MARKER).is_none() {
        return;
    }
    let mut cases = vec![("js", "node", ProxyProbe::JAVASCRIPT)];
    cases.push(("js", "bun", ProxyProbe::JAVASCRIPT));
    cases.push(("dart", "dart", ProxyProbe::DART));
    cases.push(("go", "go", ProxyProbe::GO));
    cases.push(("rb", "ruby", ProxyProbe::RUBY));
    let expected = serde_json::json!({"rows": [{"answer": 42}], "loopbackBypass": true});
    let mut unexpected = Vec::new();
    for (extension, runtime, handler) in cases {
        if !runtime_available(if runtime == "bun" { "ts" } else { extension }) {
            continue;
        }
        let answer = ProxyProbe::answer(extension, runtime, handler).await;
        if answer != expected {
            unexpected.push(format!("{runtime}: {answer}"));
        }
    }
    assert!(unexpected.is_empty(), "{unexpected:?}");
}

#[test]
fn resource_clients_connect_directly_when_a_proxy_is_configured() {
    let proxy = std::net::TcpListener::bind("127.0.0.1:0").expect("proxy stand-in");
    proxy.set_nonblocking(true).expect("non-blocking proxy");
    let address = format!("http://{}", proxy.local_addr().expect("proxy address"));
    let mut child = std::process::Command::new(std::env::current_exe().expect("test binary"));
    child.args([
        "--exact",
        "managed_resources::proxy_probe_child",
        "--nocapture",
    ]);
    child.env(ProxyProbe::MARKER, "1");
    child.env_remove("NO_PROXY").env_remove("no_proxy");
    for name in ProxyProbe::VARIABLES {
        child.env(name, &address);
    }
    let output = child.output().expect("proxy probe process");
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let forwarded = ProxyProbe::loopback_requests(&proxy);
    assert!(
        forwarded.is_empty(),
        "resource requests reached the proxy: {forwarded:?}"
    );
}
