#![allow(clippy::expect_used)]

use std::path::Path;
use std::process::{Command, Stdio};
use tachyon_contracts::{HandlerBody, HandlerBodyEncoding, HandlerRequest, HttpMethod};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
};

struct OrderProject {
    extension: &'static str,
    program: &'static str,
    files: &'static [(&'static str, &'static str)],
}

impl OrderProject {
    const PYTHON: Self = Self {
        extension: "py",
        program: "python3",
        files: &[
            (
                "server/models/Order.py",
                "from dataclasses import dataclass\n\n\n@dataclass\nclass Order:\n    name: str\n    count: int\n",
            ),
            (
                "server/controllers/OrdersController.py",
                "from typing import Annotated\nfrom models import Order\n\n\n@request_route(\"/orders\")\n@public\nclass OrdersController:\n    @post_route(\"/{id}\")\n    def post_order(self, id: Annotated[int, from_path], note: Annotated[str, from_query(\"note\")], order: Annotated[Order, from_body]) -> Order:\n        return Order(f\"{order.name}:{note}\", order.count + id)\n\n    @get_route\n    def get_orders(self) -> list[Order]:\n        return [Order(\"a\", 1)]\n",
            ),
        ],
    };

    const RUBY: Self = Self {
        extension: "rb",
        program: "ruby",
        files: &[
            (
                "server/models/Order.rb",
                "# @!attribute [r] name\n#   @return [String]\n# @!attribute [r] count\n#   @return [Integer]\nOrder = Data.define(:name, :count)\n",
            ),
            (
                "server/controllers/OrdersController.rb",
                "#[RequestRoute(\"/orders\")]\n#[Public]\nclass OrdersController\n  # @param id [Integer]\n  # @param note [String]\n  # @param order [Order]\n  # @return [Order]\n  #[PostRoute(\"/{id}\")]\n  def post_order(\n    #[FromPath]\n    id,\n    #[FromQuery(\"note\")]\n    note,\n    #[FromBody]\n    order\n  )\n    Order.new(name: \"#{order.name}:#{note}\", count: order.count + id)\n  end\n\n  # @return [Array<Order>]\n  #[GetRoute]\n  def get_orders\n    [Order.new(name: \"a\", count: 1)]\n  end\nend\n",
            ),
        ],
    };

    const LUA: Self = Self {
        extension: "lua",
        program: "lua",
        files: &[
            (
                "server/models/Order.lua",
                "---@class Order\n---@field name string\n---@field count integer\nOrder = {}\n",
            ),
            (
                "server/controllers/OrdersController.lua",
                "-- @RequestRoute(\"/orders\")\n-- @Public\n---@class OrdersController\nOrdersController = {\n  -- @Needs\n  ---@type OrderService\n  orders = nil,\n}\n\n---@param id integer\n---@param note string\n---@param order Order\n---@return Order\n-- @PostRoute(\"/{id}\")\nfunction OrdersController:post_order(\n  -- @FromPath\n  id,\n  -- @FromQuery(\"note\")\n  note,\n  -- @FromBody\n  order\n)\n  return { name = order.name .. \":\" .. note, count = order.count + id }\nend\n\n---@return Order[]\n-- @GetRoute\nfunction OrdersController:get_orders()\n  return self.orders:list()\nend\n",
            ),
            (
                "server/services/OrderService.lua",
                "-- @Service\n---@class OrderService\nOrderService = {}\n\n---@return Order[]\nfunction OrderService:list()\n  return { { name = \"a\", count = 1 } }\nend\n",
            ),
        ],
    };

    const PHP: Self = Self {
        extension: "php",
        program: "php-cgi",
        files: &[
            (
                "server/models/Order.php",
                "<?php\nclass Order {\n    public function __construct(public string $name, public int $count) {}\n}\n",
            ),
            (
                "server/controllers/OrdersController.php",
                "<?php\nuse Models\\Order;\n\n#[RequestRoute(\"/orders\")]\n#[Public]\nclass OrdersController {\n    #[PostRoute(\"/{id}\")]\n    public function postOrder(#[FromPath] int $id, #[FromQuery(\"note\")] string $note, #[FromBody] Order $order): Order {\n        return new Order($order->name . ':' . $note, $order->count + $id);\n    }\n\n    #[GetRoute]\n    public function getOrders(): array {\n        return [new Order('a', 1)];\n    }\n}\n",
            ),
        ],
    };

    const JAVA: Self = Self {
        extension: "java",
        program: "java",
        files: &[
            (
                "server/models/Order.java",
                "record Order(String name, long count) {}\n",
            ),
            (
                "server/controllers/OrdersController.java",
                "import models.Order;\n\n@RequestRoute(\"/orders\")\n@Public\nclass OrdersController {\n    @PostRoute(\"/{id}\")\n    Order postOrder(@FromPath long id, @FromQuery(\"note\") String note, @FromBody Order order) {\n        return new Order(order.name() + \":\" + note, order.count() + id);\n    }\n\n    @GetRoute\n    java.util.List<Order> getOrders() {\n        return java.util.List.of(new Order(\"a\", 1));\n    }\n}\n",
            ),
        ],
    };

    const KOTLIN: Self = Self {
        extension: "kt",
        program: "kotlinc",
        files: &[
            (
                "server/models/Order.kt",
                "data class Order(val name: String, val count: Long)\n",
            ),
            (
                "server/controllers/OrdersController.kt",
                "import models.Order\n\n@RequestRoute(\"/orders\")\n@Public\nclass OrdersController {\n    @PostRoute(\"/{id}\")\n    fun postOrder(@FromPath id: Long, @FromQuery(\"note\") note: String, @FromBody order: Order): Order {\n        return Order(order.name + \":\" + note, order.count + id)\n    }\n\n    @GetRoute\n    fun getOrders(): List<Order> {\n        return listOf(Order(\"a\", 1))\n    }\n}\n",
            ),
        ],
    };

    const GROOVY: Self = Self {
        extension: "groovy",
        program: "groovyc",
        files: &[
            (
                "server/models/Order.groovy",
                "record Order(String name, long count) {}\n",
            ),
            (
                "server/controllers/OrdersController.groovy",
                "import models.Order\n\n@RequestRoute('/orders')\n@Public\nclass OrdersController {\n    @PostRoute(\"/{id}\")\n    Order postOrder(@FromPath long id, @FromQuery('note') String note, @FromBody Order order) {\n        return new Order(\"${order.name()}:${note}\", order.count() + id)\n    }\n\n    @GetRoute\n    List<Order> getOrders() {\n        return [new Order('a', 1)]\n    }\n}\n",
            ),
        ],
    };

    const CSHARP: Self = Self {
        extension: "cs",
        program: "dotnet",
        files: &[
            (
                "server/models/Order.cs",
                "public record Order(string Name, long Count);\n",
            ),
            (
                "server/controllers/OrdersController.cs",
                "using System.Collections.Generic;\nusing Models;\n\n[RequestRoute(\"/orders\")]\n[Public]\nclass OrdersController {\n    [PostRoute(\"/{id}\")]\n    public Order PostOrder([FromPath] long id, [FromQuery(\"note\")] string note, [FromBody] Order order) {\n        return new Order(order.Name + \":\" + note, order.Count + id);\n    }\n\n    [GetRoute]\n    public List<Order> GetOrders() {\n        return new List<Order> { new Order(\"a\", 1) };\n    }\n}\n",
            ),
        ],
    };

    const FSHARP: Self = Self {
        extension: "fs",
        program: "dotnet",
        files: &[
            (
                "server/models/Order.fs",
                "type Order = { Name: string; Count: int64 }\n",
            ),
            (
                "server/controllers/OrdersController.fs",
                "open Models\n\n[<RequestRoute(\"/orders\")>]\n[<Public>]\ntype OrdersController() =\n    [<PostRoute(\"/{id}\")>]\n    member this.PostOrder([<FromPath>] id: int64, [<FromQuery(\"note\")>] note: string, [<FromBody>] order: Order) : Order =\n        { Name = order.Name + \":\" + note; Count = order.Count + id }\n\n    [<GetRoute>]\n    member this.GetOrders() : Order list =\n        [ { Name = \"a\"; Count = 1L } ]\n",
            ),
        ],
    };

    const RUST: Self = Self {
        extension: "rs",
        program: "rustc",
        files: &[
            (
                "server/models/Order.rs",
                "pub struct Order {\n    pub name: String,\n    pub count: i64,\n}\n",
            ),
            (
                "server/controllers/OrdersController.rs",
                "use crate::models::Order;\n\n#[RequestRoute(\"/orders\")]\n#[Public]\nstruct OrdersController {}\nimpl OrdersController {\n    fn new() -> Self { Self {} }\n\n    #[PostRoute(\"/{id}\")]\n    fn post_order(&self, #[FromPath] id: i64, #[FromQuery(\"note\")] note: String, #[FromBody] order: Order) -> Order {\n        Order { name: format!(\"{}:{}\", order.name, note), count: order.count + id }\n    }\n\n    #[GetRoute]\n    fn get_orders(&self) -> Vec<Order> {\n        vec![Order { name: String::from(\"a\"), count: 1 }]\n    }\n}\n",
            ),
        ],
    };

    const DART: Self = Self {
        extension: "dart",
        program: "dart",
        files: &[
            (
                "server/models/Order.dart",
                "class Order {\n  final String name;\n  final int count;\n\n  const Order(this.name, this.count);\n}\n",
            ),
            (
                "server/controllers/OrdersController.dart",
                "import 'models/Order.dart';\n\n@RequestRoute(\"/orders\")\n@Public\nclass OrdersController {\n  @PostRoute(\"/{id}\")\n  Order postOrder(@FromPath int id, @FromQuery(\"note\") String note, @FromBody Order order) {\n    return Order('${order.name}:$note', order.count + id);\n  }\n\n  @GetRoute\n  List<Order> getOrders() {\n    return [const Order('a', 1)];\n  }\n}\n",
            ),
        ],
    };

    const GO: Self = Self {
        extension: "go",
        program: "go",
        files: &[
            (
                "server/models/Order.go",
                "type Order struct {\n\tName  string `json:\"name\"`\n\tCount int64  `json:\"count\"`\n}\n",
            ),
            (
                "server/controllers/OrdersController.go",
                "// @RequestRoute(\"/orders\")\n// @Public\ntype OrdersController struct {\n}\n\n// @PostRoute(\"/{id}\")\nfunc (controller *OrdersController) PostOrder(\n// @FromPath\n id int64, \n// @FromQuery(\"note\")\n note string, \n// @FromBody\n order Order) Order {\n\treturn Order{Name: order.Name + \":\" + note, Count: order.Count + id}\n}\n\n// @GetRoute\nfunc (controller *OrdersController) GetOrders() []Order {\n\treturn []Order{{Name: \"a\", Count: 1}}\n}\n",
            ),
        ],
    };
}

fn available(program: &str) -> bool {
    let flag = match program {
        "kotlinc" => "-version",
        "go" => "version",
        "lua" => "-v",
        _ => "--version",
    };
    Command::new(program)
        .arg(flag)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn write(root: &Path, relative: &str, source: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("directories");
    std::fs::write(path, source).expect("source");
}

async fn call(
    source: &HandlerSource,
    route: &str,
    method: HttpMethod,
    arguments: Vec<serde_json::Value>,
) -> serde_json::Value {
    let supervisor =
        HandlerSupervisor::new(HandlerSupervisorOptions::default()).expect("supervisor");
    let mut request = HandlerRequest::route("languages", route, method);
    request.arguments = Some(arguments);
    request.body = Some(HandlerBody {
        encoding: HandlerBodyEncoding::Utf8,
        data: String::from("{}"),
    });
    let response = supervisor
        .invoke(source, &request, &HandlerCancellation::default())
        .await
        .expect("invocation");
    assert!(response.error.is_none(), "{:?}", response.error);
    let body = response.body.expect("body").data;
    serde_json::from_str(&body).unwrap_or_else(|_| panic!("JSON answer: {body}"))
}

async fn assert_orders(project: &OrderProject) {
    if !available(project.program) {
        eprintln!(
            "skipping {}: {} is unavailable",
            project.extension, project.program
        );
        return;
    }
    let root = tempfile::tempdir().expect("project");
    for (relative, source) in project.files {
        write(root.path(), relative, source);
    }
    let entry = format!("server/controllers/OrdersController.{}", project.extension);
    let source = HandlerSource::discover(root.path(), &entry)
        .unwrap_or_else(|failure| panic!("{}: {failure}", project.extension));
    let created = call(
        &source,
        "/orders/_id",
        HttpMethod::Post,
        vec![
            serde_json::json!(7),
            serde_json::json!("hi"),
            serde_json::json!({"name": "lamp", "count": 2}),
        ],
    )
    .await;
    assert_eq!(
        created,
        serde_json::json!({"name": "lamp:hi", "count": 9}),
        "{}",
        project.extension
    );
    let listed = call(&source, "/orders", HttpMethod::Get, Vec::new()).await;
    assert_eq!(
        listed,
        serde_json::json!([{"name": "a", "count": 1}]),
        "{}",
        project.extension
    );
}

#[tokio::test]
async fn a_python_controller_binds_arguments_and_models() {
    assert_orders(&OrderProject::PYTHON).await;
}

#[tokio::test]
async fn a_ruby_controller_binds_arguments_and_models() {
    assert_orders(&OrderProject::RUBY).await;
}

#[tokio::test]
async fn a_lua_controller_binds_arguments_and_models() {
    assert_orders(&OrderProject::LUA).await;
}

#[tokio::test]
async fn a_php_controller_binds_arguments_and_models() {
    assert_orders(&OrderProject::PHP).await;
}

#[tokio::test]
async fn a_java_controller_binds_arguments_and_models() {
    assert_orders(&OrderProject::JAVA).await;
}

#[tokio::test]
async fn a_kotlin_controller_binds_arguments_and_models() {
    assert_orders(&OrderProject::KOTLIN).await;
}

#[tokio::test]
async fn a_groovy_controller_binds_arguments_and_models() {
    assert_orders(&OrderProject::GROOVY).await;
}

#[tokio::test]
async fn a_csharp_controller_binds_arguments_and_models() {
    assert_orders(&OrderProject::CSHARP).await;
}

#[tokio::test]
async fn an_fsharp_controller_binds_arguments_and_models() {
    assert_orders(&OrderProject::FSHARP).await;
}

#[tokio::test]
async fn a_rust_controller_binds_arguments_and_models() {
    assert_orders(&OrderProject::RUST).await;
}

#[tokio::test]
async fn a_dart_controller_binds_arguments_and_models() {
    assert_orders(&OrderProject::DART).await;
}

#[tokio::test]
async fn a_go_controller_binds_arguments_and_models() {
    assert_orders(&OrderProject::GO).await;
}

#[test]
fn a_rust_model_that_fails_to_compile_is_named_in_the_error() {
    if !available("cargo") {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    for (relative, source) in OrderProject::RUST.files {
        write(root.path(), relative, source);
    }
    write(
        root.path(),
        "server/models/OrderList.rs",
        "pub struct OrderList {\n    pub orders: Vec<Order>,\n}\n",
    );
    let failure = HandlerSource::discover(root.path(), "server/controllers/OrdersController.rs")
        .expect_err("compile error");
    let rendered = failure.to_string();
    assert!(rendered.contains("Handler failed to compile"), "{rendered}");
    assert!(
        rendered.contains("Source: 'server/models/OrderList.rs'"),
        "{rendered}"
    );
}
