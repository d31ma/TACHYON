#![allow(clippy::expect_used)]

use crate::support::yon_layout;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tachyon_contracts::{HandlerRequest, HttpMethod};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
    TopicDeliveries,
};
use tokio_util::sync::CancellationToken;

struct TopicFixture;

struct TypedTopic {
    extension: &'static str,
    program: &'static str,
    tag: &'static str,
    models: [(&'static str, &'static str); 2],
    publisher: &'static str,
    listener_path: &'static str,
    listener: &'static str,
}

impl TypedTopic {
    const JAVA: Self = Self {
        extension: "java",
        program: "java",
        tag: "java",
        models: [
            (
                "server/models/OrderPlaced.java",
                "@Topic\nrecord OrderPlaced(String id) {}\n",
            ),
            (
                "server/models/OrderSeen.java",
                "@Topic\nrecord OrderSeen(String by) {}\n",
            ),
        ],
        publisher: r#"
import models.OrderPlaced;
import models.OrderSeen;
@Service()
@NeedsAll()
class OrderService {
  ApplicationEventPublisher applicationEventPublisher;
  void place() { applicationEventPublisher.publishEvent(new OrderPlaced("A-1")); }
  @OnEvent
  OrderSeen seen(OrderPlaced order) { return new OrderSeen("local"); }
}
@RequestRoute("/orders")
@Public
@NeedsAll()
class OrdersController {
  OrderService orderService;
  @GetRoute
  Object getOrder() { orderService.place(); return java.util.Map.of("placed", true); }
}
"#,
        listener_path: "server/controllers/JavaAuditController.java",
        listener: r#"
import models.OrderPlaced;
import models.OrderSeen;
@Service()
class JavaAuditService {
  @OnEvent
  OrderSeen seen(OrderPlaced order) { return new OrderSeen("java:" + order.id()); }
}
@RequestRoute("/java-audit")
@Public
@NeedsAll()
class JavaAuditController {
  JavaAuditService javaAuditService;
  @GetRoute
  String getAudit() { return "ok"; }
}
"#,
    };

    const KOTLIN: Self = Self {
        extension: "kt",
        program: "kotlinc",
        tag: "kt",
        models: [
            (
                "server/models/OrderPlaced.kt",
                "@Topic\ndata class OrderPlaced(val id: String)\n",
            ),
            (
                "server/models/OrderSeen.kt",
                "@Topic\ndata class OrderSeen(val by: String)\n",
            ),
        ],
        publisher: r#"
import models.OrderPlaced
import models.OrderSeen
@Service()
@NeedsAll()
class OrderService {
  lateinit var applicationEventPublisher: ApplicationEventPublisher
  fun place() { applicationEventPublisher.publishEvent(OrderPlaced("A-1")) }
  @OnEvent
  fun seen(order: OrderPlaced): OrderSeen { return OrderSeen("local") }
}
@RequestRoute("/orders")
@Public
@NeedsAll()
class OrdersController {
  lateinit var orderService: OrderService
  @GetRoute
  fun getOrder(): Any? { orderService.place(); return mapOf("placed" to true) }
}
"#,
        listener_path: "server/controllers/KtAuditController.kt",
        listener: r#"
import models.OrderPlaced
import models.OrderSeen
@Service()
class KtAuditService {
  @OnEvent
  fun seen(order: OrderPlaced): OrderSeen { return OrderSeen("kt:" + order.id) }
}
@RequestRoute("/kt-audit")
@Public
@NeedsAll()
class KtAuditController {
  lateinit var ktAuditService: KtAuditService
  @GetRoute
  fun getAudit(): String { return "ok" }
}
"#,
    };

    const CSHARP: Self = Self {
        extension: "cs",
        program: "dotnet",
        tag: "cs",
        models: [
            (
                "server/models/OrderPlaced.cs",
                "[Topic]\npublic record OrderPlaced(string Id);\n",
            ),
            (
                "server/models/OrderSeen.cs",
                "[Topic]\npublic record OrderSeen(string By);\n",
            ),
        ],
        publisher: r#"
[Service()]
[NeedsAll()]
class OrderService {
  readonly ApplicationEventPublisher _applicationEventPublisher;
  public void Place() { _applicationEventPublisher.PublishEvent(new Models.OrderPlaced("A-1")); }
  [OnEvent]
  public Models.OrderSeen Seen(Models.OrderPlaced order) { return new Models.OrderSeen("local"); }
}
[RequestRoute("/orders")]
[Public]
[NeedsAll()]
class OrdersController {
  readonly OrderService _orderService;
  [GetRoute]
  public object GetOrder() { _orderService.Place(); return new { placed = true }; }
}
"#,
        listener_path: "server/controllers/CsAuditController.cs",
        listener: r#"
[Service()]
class CsAuditService {
  [OnEvent]
  public Models.OrderSeen Seen(Models.OrderPlaced order) { return new Models.OrderSeen("cs:" + order.Id); }
}
[RequestRoute("/cs-audit")]
[Public]
[NeedsAll()]
class CsAuditController {
  readonly CsAuditService _csAuditService;
  [GetRoute]
  public string GetAudit() { return "ok"; }
}
"#,
    };

    const DART: Self = Self {
        extension: "dart",
        program: "dart",
        tag: "dart",
        models: [
            (
                "server/models/OrderPlaced.dart",
                "@Topic\nclass OrderPlaced {\n  final String id;\n\n  const OrderPlaced(this.id);\n}\n",
            ),
            (
                "server/models/OrderSeen.dart",
                "@Topic\nclass OrderSeen {\n  final String by;\n\n  const OrderSeen(this.by);\n}\n",
            ),
        ],
        publisher: r#"
import 'models/OrderPlaced.dart';
import 'models/OrderSeen.dart';
@Service()
class OrderService {
  @Needs()
  final ApplicationEventPublisher applicationEventPublisher;

  void place() {
    applicationEventPublisher.publishEvent(const OrderPlaced('A-1'));
  }

  @OnEvent
  OrderSeen seen(OrderPlaced order) {
    return const OrderSeen('local');
  }
}
@RequestRoute("/orders")
@Public
class OrdersController {
  @Needs()
  final OrderService orderService;

  @GetRoute
  Object getOrder() {
    orderService.place();
    return {'placed': true};
  }
}
"#,
        listener_path: "server/controllers/DartAuditController.dart",
        listener: r#"
import 'models/OrderPlaced.dart';
import 'models/OrderSeen.dart';
@Service()
class DartAuditService {
  @OnEvent
  OrderSeen seen(OrderPlaced order) {
    return OrderSeen('dart:${order.id}');
  }
}
@RequestRoute("/dart-audit")
@Public
class DartAuditController {
  @Needs()
  final DartAuditService dartAuditService;

  @GetRoute
  String getAudit() {
    return 'ok';
  }
}
"#,
    };

    const RUST: Self = Self {
        extension: "rs",
        program: "cargo",
        tag: "rs",
        models: [
            (
                "server/models/OrderPlaced.rs",
                "#[Topic]\npub struct OrderPlaced {\n    pub id: String,\n}\n",
            ),
            (
                "server/models/OrderSeen.rs",
                "#[Topic]\npub struct OrderSeen {\n    pub by: String,\n}\n",
            ),
        ],
        publisher: r#"
#[Service()]
#[NeedsAll()]
struct OrderService { application_event_publisher: std::sync::Arc<ApplicationEventPublisher> }
impl OrderService {
  fn place(&self) {
    self.application_event_publisher.publish_event(crate::models::OrderPlaced { id: String::from("A-1") }, false);
  }
  #[OnEvent]
  fn seen(&self, order: crate::models::OrderPlaced) -> crate::models::OrderSeen {
    let _ = order;
    crate::models::OrderSeen { by: String::from("local") }
  }
}
#[RequestRoute("/orders")]
#[Public]
#[NeedsAll()]
struct OrdersController { order_service: std::sync::Arc<OrderService> }
impl OrdersController {
  #[GetRoute]
  fn get_order(&self) -> Json {
    self.order_service.place();
    Json::map([("placed", Json::from(true))])
  }
}
"#,
        listener_path: "server/controllers/RsAuditController.rs",
        listener: r#"
#[Service()]
struct RsAuditService;
impl RsAuditService {
  fn new() -> Self { Self }
  #[OnEvent]
  fn seen(&self, order: crate::models::OrderPlaced) -> crate::models::OrderSeen {
    crate::models::OrderSeen { by: format!("rs:{}", order.id) }
  }
}
#[RequestRoute("/rs-audit")]
#[Public]
#[NeedsAll()]
struct RsAuditController { rs_audit_service: std::sync::Arc<RsAuditService> }
impl RsAuditController {
  #[GetRoute]
  fn get_audit(&self) -> Json {
    Json::from("ok")
  }
}
"#,
    };

    const PHP: Self = Self {
        extension: "php",
        program: "php-cgi",
        tag: "php",
        models: [
            (
                "server/models/OrderPlaced.php",
                "<?php\n#[Topic]\nclass OrderPlaced {\n    public function __construct(public string $id) {}\n}\n",
            ),
            (
                "server/models/OrderSeen.php",
                "<?php\n#[Topic]\nclass OrderSeen {\n    public function __construct(public string $by) {}\n}\n",
            ),
        ],
        publisher: r#"<?php
#[Service()]
#[NeedsAll()]
class OrderService {
  private ApplicationEventPublisher $applicationEventPublisher;
  function place() { $this->applicationEventPublisher->publishEvent(new \Models\OrderPlaced('A-1')); }
  #[OnEvent]
  function seen(\Models\OrderPlaced $order): \Models\OrderSeen { return new \Models\OrderSeen('local'); }
}
#[RequestRoute("/orders")]
#[Public]
#[NeedsAll()]
class OrdersController {
  public OrderService $orderService;
  #[GetRoute]
  function getOrder() { $this->orderService->place(); return ["placed" => true]; }
}
"#,
        listener_path: "server/controllers/PhpAuditController.php",
        listener: r#"<?php
#[Service()]
class PhpAuditService {
  #[OnEvent]
  function seen(\Models\OrderPlaced $order): \Models\OrderSeen { return new \Models\OrderSeen('php:' . $order->id); }
}
#[RequestRoute("/php-audit")]
#[Public]
#[NeedsAll()]
class PhpAuditController {
  public PhpAuditService $phpAuditService;
  #[GetRoute]
  function getAudit() { return "ok"; }
}
"#,
    };

    const LUA: Self = Self {
        extension: "lua",
        program: "lua",
        tag: "lua",
        models: [
            (
                "server/models/OrderPlaced.lua",
                "-- @Topic\n---@class OrderPlaced\n---@field id string\nOrderPlaced = {}\n",
            ),
            (
                "server/models/OrderSeen.lua",
                "-- @Topic\n---@class OrderSeen\n---@field by string\nOrderSeen = {}\n",
            ),
        ],
        publisher: r#"-- @RequestRoute("/orders")
-- @Public
---@class OrdersController
OrdersController = {
  ---@type ApplicationEventPublisher
  application_event_publisher = nil,
}

---@return table
-- @GetRoute
function OrdersController:get_order()
  self.application_event_publisher:publish_event(setmetatable({ id = "A-1" }, OrderPlaced))
  return { placed = true }
end

---@param order OrderPlaced
---@return OrderSeen
-- @OnEvent
function OrdersController:seen(order)
  return setmetatable({ by = "local" }, OrderSeen)
end
"#,
        listener_path: "server/controllers/LuaAuditController.lua",
        listener: r#"-- @RequestRoute("/lua-audit")
-- @Public
---@class LuaAuditController
LuaAuditController = {}

---@param order OrderPlaced
---@return OrderSeen
-- @OnEvent
function LuaAuditController:seen(order)
  return setmetatable({ by = "lua:" .. order.id }, OrderSeen)
end

---@return string
-- @GetRoute
function LuaAuditController:get_audit()
  return "ok"
end
"#,
    };

    async fn crosses_both_ways(&self) {
        if !TopicFixture::available(self.program) {
            eprintln!(
                "Skipping the {} topic events: {} is unavailable",
                self.extension, self.program
            );
            return;
        }
        let received = format!("{}:A-1", self.tag);
        let root = tempfile::tempdir().expect("fixture");
        TopicFixture::write_models(root.path(), &TopicFixture::PYTHON_MODELS);
        TopicFixture::write_models(root.path(), &self.models);
        let publisher = TopicFixture::write(
            root.path(),
            &format!("server/controllers/OrdersController.{}", self.extension),
            self.publisher,
        );
        let listeners = vec![
            TopicFixture::write(
                root.path(),
                "server/controllers/PyAuditController.py",
                TopicFixture::PYTHON_LISTENER,
            ),
            TopicFixture::write(root.path(), self.listener_path, self.listener),
        ];
        let mut expected = vec![received.as_str(), "local", "py:A-1"];
        expected.sort_unstable();
        let seen =
            TopicFixture::publish_and_wait(root.path(), publisher, listeners, &expected).await;
        assert_eq!(seen, expected, "{} publishes", self.extension);

        let root = tempfile::tempdir().expect("fixture");
        TopicFixture::write_models(root.path(), &TopicFixture::PYTHON_MODELS);
        TopicFixture::write_models(root.path(), &self.models);
        let publisher = TopicFixture::write(
            root.path(),
            "server/controllers/OrdersController.py",
            TopicFixture::PYTHON_PUBLISHER,
        );
        let listener = TopicFixture::write(root.path(), self.listener_path, self.listener);
        let mut expected = vec![received.as_str(), "local"];
        expected.sort_unstable();
        let seen =
            TopicFixture::publish_and_wait(root.path(), publisher, vec![listener], &expected).await;
        assert_eq!(seen, expected, "{} receives", self.extension);
    }
}

impl TopicFixture {
    const JAVASCRIPT_PUBLISHER: &'static str = r#"
/**
 * @Topic
 * @typedef {Object} OrderPlaced
 * @property {string} id
 */
/**
 * @Topic
 * @typedef {Object} OrderSeen
 * @property {string} by
 */
@Service()
@NeedsAll()
class OrderService {
  applicationEventPublisher;
  place() {
    /** @type {OrderPlaced} */
    const placed = {id: 'A-1'};
    this.applicationEventPublisher.publishEvent(placed);
  }
  /**
   * @param {OrderPlaced} order
   * @returns {OrderSeen}
   */
  @OnEvent
  seen(order) { return {by: 'local'}; }
}
@RequestRoute("/orders")
@Public
@NeedsAll()
class OrdersController {
  orderService;
  @GetRoute
  getOrder() { this.orderService.place(); return {placed: true}; }
}
"#;

    const JAVASCRIPT_LISTENER: &'static str = r#"
/**
 * @Topic
 * @typedef {Object} OrderPlaced
 * @property {string} id
 */
/**
 * @Topic
 * @typedef {Object} OrderSeen
 * @property {string} by
 */
@Service()
class JsAuditService {
  /**
   * @param {OrderPlaced} order
   * @returns {OrderSeen}
   */
  @OnEvent
  seen(order) { return {by: 'js:' + order.id}; }
}
@RequestRoute("/audit")
@Public
@NeedsAll()
class JsAuditController {
  jsAuditService;
  @GetRoute
  getAudit() { return 'ok'; }
}
"#;

    const PYTHON_MODELS: [(&'static str, &'static str); 2] = [
        (
            "server/models/OrderPlaced.py",
            "from dataclasses import dataclass\n\n\n@topic\n@dataclass\nclass OrderPlaced:\n    id: str\n",
        ),
        (
            "server/models/OrderSeen.py",
            "from dataclasses import dataclass\n\n\n@topic\n@dataclass\nclass OrderSeen:\n    by: str\n",
        ),
    ];

    const PYTHON_PUBLISHER: &'static str = r#"
from models import OrderPlaced, OrderSeen
@service()
@needs_all()
class OrderService:
    application_event_publisher: ApplicationEventPublisher
    def place(self):
        self.application_event_publisher.publish_event(OrderPlaced("A-1"))
    @on_event
    def seen(self, order: OrderPlaced) -> OrderSeen:
        return OrderSeen("local")
@request_route("/orders")
@public
@needs_all()
class OrdersController:
    order_service: OrderService
    @get_route
    def get_order(self):
        self.order_service.place()
        return {"placed": True}
"#;

    const PYTHON_LISTENER: &'static str = r#"
from models import OrderPlaced, OrderSeen
@service()
class PyAuditService:
    @on_event
    def seen(self, order: OrderPlaced) -> OrderSeen:
        return OrderSeen("py:" + order.id)
@request_route("/py-audit")
@public
@needs_all()
class PyAuditController:
    py_audit_service: PyAuditService
    @get_route
    def get_audit(self):
        return "ok"
"#;

    const GO_MODELS: [(&'static str, &'static str); 2] = [
        (
            "server/models/OrderPlaced.go",
            "package main\n\n// @Topic\ntype OrderPlaced struct {\n\tID string `json:\"id\"`\n}\n",
        ),
        (
            "server/models/OrderSeen.go",
            "package main\n\n// @Topic\ntype OrderSeen struct {\n\tBy string `json:\"by\"`\n}\n",
        ),
    ];

    const GO_LISTENER: &'static str = r#"
// @Service()
type GoAuditService struct{}

// @OnEvent
func (audit *GoAuditService) Seen(order OrderPlaced) OrderSeen {
	return OrderSeen{By: "go:" + order.ID}
}
// @RequestRoute("/go-audit")
// @Public
// @NeedsAll()
type GoAuditController struct {
	audit *GoAuditService
}

// @GetRoute
func (controller *GoAuditController) GetAudit() any {
	return "ok"
}
"#;

    const RUBY_MODELS: [(&'static str, &'static str); 2] = [
        (
            "server/models/OrderPlaced.rb",
            "#[Topic]\n# @!attribute [r] id\n#   @return [String]\nOrderPlaced = Data.define(:id)\n",
        ),
        (
            "server/models/OrderSeen.rb",
            "#[Topic]\n# @!attribute [r] by\n#   @return [String]\nOrderSeen = Data.define(:by)\n",
        ),
    ];

    const RUBY_LISTENER: &'static str = r#"#[Service]
class RbAuditService
  # @param order [OrderPlaced]
  # @return [OrderSeen]
  #[OnEvent]
  def seen(order)
    OrderSeen.new(by: "rb:#{order.id}")
  end
end
#[RequestRoute("/rb-audit")]
#[Public]
class RbAuditController
  #[Needs]
  attr_reader :rb_audit_service

  #[GetRoute]
  def get_audit
    "ok"
  end
end
"#;

    fn available(program: &str) -> bool {
        let paths = std::env::var_os("PATH").unwrap_or_default();
        std::env::split_paths(&paths).any(|directory| directory.join(program).is_file())
    }

    fn write_models(root: &Path, models: &[(&str, &str)]) {
        for (path, source) in models {
            let path = root.join(path);
            std::fs::create_dir_all(path.parent().expect("models folder")).expect("models folder");
            std::fs::write(path, source).expect("model source");
        }
    }

    fn write(root: &Path, relative: &str, source: &str) -> HandlerSource {
        let is_lua = Path::new(relative)
            .extension()
            .is_some_and(|extension| extension == "lua");
        let entry = if is_lua {
            Self::write_models(root, &[(relative, source)]);
            relative.to_owned()
        } else {
            yon_layout::write_fixture(root, relative, source)
        };
        HandlerSource::discover(root, &entry).expect("topic handler discovery")
    }

    fn seen(root: &Path) -> Vec<String> {
        let log =
            std::fs::read_to_string(root.join("data/topics/order-seen.jsonl")).unwrap_or_default();
        let mut seen: Vec<String> = log
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .filter_map(|value| value["by"].as_str().map(str::to_owned))
            .collect();
        seen.sort();
        seen
    }

    async fn publish_and_wait(
        root: &Path,
        publisher: HandlerSource,
        listeners: Vec<HandlerSource>,
        expected: &[&str],
    ) -> Vec<String> {
        let mut options = HandlerSupervisorOptions::default();
        options.runtimes.javascript = "bun".into();
        let supervisor = Arc::new(HandlerSupervisor::new(options).expect("supervisor"));
        let sources: Vec<HandlerSource> = std::iter::once(publisher.clone())
            .chain(listeners)
            .collect();
        let deliveries =
            TopicDeliveries::connect(root, &supervisor, sources.iter()).expect("topic bus");
        let cancellation = CancellationToken::new();
        let running = tokio::spawn(deliveries.run(Arc::clone(&supervisor), cancellation.clone()));
        let request = HandlerRequest::route("topic_publish", "/orders", HttpMethod::Get);
        let response = supervisor
            .invoke(&publisher, &request, &HandlerCancellation::default())
            .await
            .expect("publish invocation");
        assert_eq!(response.status, 200, "{:?}", response.error);
        for _ in 0..600 {
            if Self::seen(root).len() >= expected.len() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
        cancellation.cancel();
        let _stopped = running.await;
        Self::seen(root)
    }
}

#[tokio::test]
async fn a_javascript_topic_event_reaches_another_process_once() {
    let root = tempfile::tempdir().expect("fixture");
    let publisher = TopicFixture::write(
        root.path(),
        "server/controllers/OrdersController.js",
        TopicFixture::JAVASCRIPT_PUBLISHER,
    );
    let listener = TopicFixture::write(
        root.path(),
        "server/controllers/JsAuditController.js",
        TopicFixture::JAVASCRIPT_LISTENER,
    );
    let expected = ["js:A-1", "local"];
    let seen =
        TopicFixture::publish_and_wait(root.path(), publisher, vec![listener], &expected).await;
    assert_eq!(seen, expected);
    let placed = std::fs::read_to_string(root.path().join("data/topics/order-placed.jsonl"))
        .expect("order-placed log");
    assert_eq!(placed, "{\"id\":\"A-1\"}\n");
}

#[tokio::test]
async fn a_python_topic_event_reaches_javascript_and_python_listeners() {
    let root = tempfile::tempdir().expect("fixture");
    TopicFixture::write_models(root.path(), &TopicFixture::PYTHON_MODELS);
    let publisher = TopicFixture::write(
        root.path(),
        "server/controllers/OrdersController.py",
        TopicFixture::PYTHON_PUBLISHER,
    );
    let listeners = vec![
        TopicFixture::write(
            root.path(),
            "server/controllers/JsAuditController.js",
            TopicFixture::JAVASCRIPT_LISTENER,
        ),
        TopicFixture::write(
            root.path(),
            "server/controllers/PyAuditController.py",
            TopicFixture::PYTHON_LISTENER,
        ),
    ];
    let expected = ["js:A-1", "local", "py:A-1"];
    let seen = TopicFixture::publish_and_wait(root.path(), publisher, listeners, &expected).await;
    assert_eq!(seen, expected);
}

#[tokio::test]
async fn a_python_topic_event_reaches_a_go_listener() {
    if !TopicFixture::available("go") {
        eprintln!("Skipping the Go topic listener: go is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("fixture");
    TopicFixture::write_models(root.path(), &TopicFixture::PYTHON_MODELS);
    TopicFixture::write_models(root.path(), &TopicFixture::GO_MODELS);
    let publisher = TopicFixture::write(
        root.path(),
        "server/controllers/OrdersController.py",
        TopicFixture::PYTHON_PUBLISHER,
    );
    let listener = TopicFixture::write(
        root.path(),
        "server/controllers/GoAuditController.go",
        TopicFixture::GO_LISTENER,
    );
    let expected = ["go:A-1", "local"];
    let seen =
        TopicFixture::publish_and_wait(root.path(), publisher, vec![listener], &expected).await;
    assert_eq!(seen, expected);
}

#[tokio::test]
async fn a_python_topic_event_reaches_a_ruby_listener() {
    if !TopicFixture::available("ruby") {
        eprintln!("Skipping the Ruby topic listener: ruby is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("fixture");
    TopicFixture::write_models(root.path(), &TopicFixture::PYTHON_MODELS);
    TopicFixture::write_models(root.path(), &TopicFixture::RUBY_MODELS);
    let publisher = TopicFixture::write(
        root.path(),
        "server/controllers/OrdersController.py",
        TopicFixture::PYTHON_PUBLISHER,
    );
    let listener = TopicFixture::write(
        root.path(),
        "server/controllers/RbAuditController.rb",
        TopicFixture::RUBY_LISTENER,
    );
    let expected = ["local", "rb:A-1"];
    let seen =
        TopicFixture::publish_and_wait(root.path(), publisher, vec![listener], &expected).await;
    assert_eq!(seen, expected);
}

#[tokio::test]
async fn java_topic_events_cross_to_python_and_back() {
    TypedTopic::JAVA.crosses_both_ways().await;
}

#[tokio::test]
async fn kotlin_topic_events_cross_to_python_and_back() {
    TypedTopic::KOTLIN.crosses_both_ways().await;
}

#[tokio::test]
async fn csharp_topic_events_cross_to_python_and_back() {
    TypedTopic::CSHARP.crosses_both_ways().await;
}

#[tokio::test]
async fn dart_topic_events_cross_to_python_and_back() {
    TypedTopic::DART.crosses_both_ways().await;
}

#[tokio::test]
async fn rust_topic_events_cross_to_python_and_back() {
    TypedTopic::RUST.crosses_both_ways().await;
}

#[tokio::test]
async fn php_topic_events_cross_to_python_and_back() {
    TypedTopic::PHP.crosses_both_ways().await;
}

#[tokio::test]
async fn lua_topic_events_cross_to_python_and_back() {
    TypedTopic::LUA.crosses_both_ways().await;
}
