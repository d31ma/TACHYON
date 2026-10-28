#![allow(clippy::expect_used)]

use std::path::Path;

struct RequestPartsProject {
    language: &'static str,
    program: &'static str,
    version_flag: &'static str,
    unit: &'static str,
    files: &'static [(&'static str, &'static str)],
}

impl RequestPartsProject {
    const DART: Self = Self {
        language: "Dart",
        program: "dart",
        version_flag: "--version",
        unit: "client/tests/OrderServiceTest.dart",
        files: &[
            (
                "client/models/OrderEcho.dart",
                "class OrderEcho {\n  final String path;\n  final String view;\n\n  const OrderEcho(this.path, this.view);\n}\n",
            ),
            (
                "client/remotes/OrdersClient.dart",
                "@HTTPClient(\"/api\")\nabstract interface class OrdersClient {\n  @GetRequest(\"/orders/{id}\")\n  Future<OrderEcho> getOrder(@PathVariable int id, @RequestParam(\"view\") String view);\n}\n",
            ),
            (
                "client/setups/OrdersSetup.dart",
                "@Setup\nclass OrdersSetup {\n  static BehaviorAdapters configure(BehaviorExecution execution) {\n    return BehaviorAdapters();\n  }\n}\n",
            ),
            (
                "client/services/OrderService.dart",
                "@Service\nclass OrderService {\n  @Needs\n  final OrdersClient ordersClient;\n\n  Future<OrderEcho> read() => ordersClient.getOrder(7, 'short');\n}\n",
            ),
            (
                "client/tests/OrderTestSetup.dart",
                "@TestSetup\nclass OrderTestSetup {\n  static BehaviorAdapters configure(BehaviorExecution execution) {\n    return BehaviorAdapters(http: {\n      'orders': BehaviorHttpAdapter(exchange: _echo),\n    });\n  }\n\n  static Future<Object?> _echo(Object? request, BehaviorExecution running) async {\n    final fields = request as Map;\n    final query = fields['query'] as Map;\n    return <String, Object?>{'path': fields['path'], 'view': query['view']};\n  }\n}\n",
            ),
            (
                "client/tests/OrderServiceTest.dart",
                "@Test(OrderTestSetup, {\"partition\":\"tests\"})\nclass OrderServiceTest {\n  @Fake\n  OrdersClient? ordersClient;\n\n  Future<void> testGetFillsThePathVariableAndTheRequestParam() async {\n    ordersClient = OrdersClient(behaviors!);\n    final service = construct<OrderService>(OrderService);\n    final echo = await service.read();\n    Assert.equals('/api/orders/7', echo.path);\n    Assert.equals('short', echo.view);\n  }\n}\n",
            ),
        ],
    };

    const RUST: Self = Self {
        language: "Rust",
        program: "rustc",
        version_flag: "--version",
        unit: "client/tests/OrderServiceTest.rs",
        files: &[
            (
                "client/models/OrderEcho.rs",
                "#[derive(Clone, Debug)]\npub struct OrderEcho {\n    pub path: String,\n    pub view: String,\n}\n",
            ),
            (
                "client/remotes/OrdersClient.rs",
                "use crate::models::OrderEcho;\n\n#[HTTPClient(\"/api\")]\ntrait OrdersClient {\n    #[GetRequest(\"/orders/{id}\")]\n    async fn get_order(&self, #[PathVariable] id: i32, #[RequestParam(\"view\")] view: String) -> BehaviorResult<OrderEcho>;\n}\n",
            ),
            (
                "client/setups/OrdersSetup.rs",
                "#[Setup]\nstruct OrdersSetup {}\nimpl OrdersSetup {\n    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {\n        Ok(BehaviorConfiguration::default())\n    }\n}\n",
            ),
            (
                "client/services/OrderService.rs",
                "#[Service]\npub struct OrderService {\n    #[Needs]\n    orders_client: std::sync::Arc<OrdersClient>,\n}\n\nimpl OrderService {\n    pub async fn read(&self) -> BehaviorResult<OrderEcho> {\n        self.orders_client.get_order(7, String::from(\"short\")).await\n    }\n}\n",
            ),
            (
                "client/tests/OrderTestSetup.rs",
                "#[TestSetup]\nstruct OrderTestSetup {}\nimpl OrderTestSetup {\n    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {\n        struct OrdersEcho;\n        impl BehaviorHttpAdapter for OrdersEcho {\n            fn exchange(&self, request: &BehaviorHttpRequest, _execution: &BehaviorExecution) -> BehaviorResult<BehaviorValue> {\n                let view = request.query.get(\"view\").cloned().unwrap_or_else(|| BehaviorValue::from(\"missing\"));\n                Ok(BehaviorValue::object([(\"path\", BehaviorValue::from(request.path.as_str())), (\"view\", view)]))\n            }\n        }\n\n        let mut configuration = BehaviorConfiguration::default();\n        configuration.http.insert(\"orders\".into(), std::sync::Arc::new(OrdersEcho));\n        Ok(configuration)\n    }\n}\n",
            ),
            (
                "client/tests/OrderServiceTest.rs",
                "#[Test(OrderTestSetup, {\"partition\":\"tests\"})]\nstruct OrderServiceTest {\n    behaviors: std::cell::RefCell<Option<BehaviorContext>>,\n    #[Fake]\n    orders_client: std::cell::RefCell<Option<std::sync::Arc<OrdersClient>>>,\n}\nimpl OrderServiceTest {\n    fn new() -> Self {\n        Self {\n            behaviors: std::cell::RefCell::new(None),\n            orders_client: std::cell::RefCell::new(None),\n        }\n    }\n\n    async fn testGetFillsThePathVariableAndTheRequestParam(&self) {\n        let service: OrderService = self.construct();\n        let echo = service.read().await.ok();\n        Assert::is_true(echo.is_some());\n        let echo = echo.unwrap_or_else(|| OrderEcho { path: String::new(), view: String::new() });\n        Assert::equals(\"/api/orders/7\", echo.path);\n        Assert::equals(\"short\", echo.view);\n    }\n}\n",
            ),
        ],
    };

    const CSHARP: Self = Self {
        language: "C#",
        program: "dotnet",
        version_flag: "--version",
        unit: "client/tests/OrderServiceTest.cs",
        files: &[
            (
                "client/models/OrderEcho.cs",
                "public record OrderEcho(string Path, string View);\n",
            ),
            (
                "client/remotes/OrdersClient.cs",
                "[HTTPClient({\"path\":\"/api\",\"adapter\":\"orders\",\"setup\":\"OrdersSetup\"})]\npublic interface OrdersClient\n{\n    [GetRequest(\"/orders/{id}\")]\n    System.Threading.Tasks.Task<Models.OrderEcho> getOrder([PathVariable] long id, [RequestParam(\"view\")] string view);\n}\n",
            ),
            (
                "client/setups/OrdersSetup.cs",
                "[Setup]\npublic sealed class OrdersSetup\n{\n    public static BehaviorAdapters configure(BehaviorExecution execution)\n    {\n        execution.Check();\n        return new BehaviorAdapters();\n    }\n}\n",
            ),
            (
                "client/services/OrderService.cs",
                "[Service]\npublic sealed class OrderService\n{\n    [Needs]\n    private readonly OrdersClient _ordersClient;\n\n    public System.Threading.Tasks.Task<Models.OrderEcho> Read()\n    {\n        return _ordersClient.getOrder(7, \"short\");\n    }\n}\n",
            ),
            (
                "client/delegates/OrdersEchoDelegate.cs",
                "[Delegate]\npublic sealed class OrdersEchoDelegate : BehaviorHTTPAdapter\n{\n    public System.Threading.Tasks.Task<BehaviorJSON> exchange(BehaviorJSON request, BehaviorExecution execution)\n    {\n        execution.Check();\n        var echo = new System.Collections.Generic.Dictionary<string, BehaviorJSON>\n        {\n            [\"path\"] = request.Get(\"path\"),\n            [\"view\"] = request.Get(\"query\").Get(\"view\"),\n        };\n        return System.Threading.Tasks.Task.FromResult(BehaviorJSON.Object(echo));\n    }\n}\n",
            ),
            (
                "client/tests/OrderTestSetup.cs",
                "[TestSetup]\npublic sealed class OrderTestSetup\n{\n    public static BehaviorAdapters configure(BehaviorExecution execution)\n    {\n        execution.Check();\n        var adapters = new BehaviorAdapters();\n        adapters.http[\"orders\"] = new OrdersEchoDelegate();\n        return adapters;\n    }\n}\n",
            ),
            (
                "client/tests/OrderServiceTest.cs",
                "[Test(typeof(OrderTestSetup), {\"partition\":\"tests\"})]\npublic sealed class OrderServiceTest\n{\n    [Fake]\n    public OrdersClient _ordersClient = null;\n\n    void testGetFillsThePathVariableAndTheRequestParam()\n    {\n        using var signal = new BehaviorSignal();\n        using var context = Behaviors.create(typeof(OrderTestSetup), \"orders\", signal);\n        _ordersClient = new OrdersClient(context);\n        var echo = construct<OrderService>().Read().GetAwaiter().GetResult();\n        Assert.equals(\"/api/orders/7\", echo.Path);\n        Assert.equals(\"short\", echo.View);\n    }\n}\n",
            ),
        ],
    };

    const KOTLIN: Self = Self {
        language: "Kotlin",
        program: "kotlinc",
        version_flag: "-version",
        unit: "client/tests/OrderServiceTest.kt",
        files: &[
            (
                "client/models/OrderEcho.kt",
                "data class OrderEcho(val path: String, val view: String)\n",
            ),
            (
                "client/remotes/OrdersClient.kt",
                "import models.OrderEcho\n\n@HTTPClient(\"/api\")\ninterface OrdersClient {\n  @GetRequest(\"/orders/{id}\")\n  suspend fun getOrder(@PathVariable id: Long, @RequestParam(\"view\") view: String): OrderEcho\n}\n",
            ),
            (
                "client/services/OrderService.kt",
                "import models.OrderEcho\n\n@Service\nclass OrderService {\n  @Needs\n  private val ordersClient: OrdersClient\n\n  suspend fun read(): OrderEcho = ordersClient.getOrder(7L, \"short\")\n}\n",
            ),
            (
                "client/tests/OrderTestSetup.kt",
                "@TestSetup\nclass OrderTestSetup {\n  companion object {\n    fun configure(execution: Behaviors.Execution): Behaviors.Configuration =\n      Behaviors.Configuration(http = mapOf(\"orders\" to Behaviors.HttpAdapter { request, _ ->\n        mapOf(\"path\" to request[\"path\"], \"view\" to (request[\"query\"] as Map<*, *>)[\"view\"])\n      }))\n  }\n}\n",
            ),
            (
                "client/tests/OrderServiceTest.kt",
                "@Test(OrderTestSetup::class, {\"partition\":\"tests\"})\nclass OrderServiceTest {\n  @Fake\n  var ordersClient: OrdersClient? = null\n\n  suspend fun testGetFillsThePathVariableAndTheRequestParam() {\n    val echo = construct(OrderService::class).read()\n    Assert.equals(\"/api/orders/7\", echo.path)\n    Assert.equals(\"short\", echo.view)\n  }\n}\n",
            ),
        ],
    };

    const SWIFT: Self = Self {
        language: "Swift",
        program: "swiftc",
        version_flag: "--version",
        unit: "client/tests/OrderServiceTest.swift",
        files: &[
            (
                "client/models/OrderEcho.swift",
                "struct OrderEcho: Codable {\n  let path: String\n  let view: String\n}\n",
            ),
            (
                "client/remotes/OrdersClient.swift",
                "@HTTPClient(\"/api\")\nprotocol OrdersClient {\n  @GetRequest(\"/orders/{id}\")\n  func getOrder(@PathVariable id: Int, @RequestParam(\"view\") view: String) async throws -> OrderEcho\n}\n",
            ),
            (
                "client/setups/OrdersSetup.swift",
                "@Setup\nfinal class OrdersSetup {\n  static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters {\n    BehaviorAdapters()\n  }\n}\n",
            ),
            (
                "client/services/OrderService.swift",
                "@Service\nfinal class OrderService {\n  @Needs\n  private let ordersClient: OrdersClient\n\n  func read() async throws -> OrderEcho {\n    return try await ordersClient.getOrder(id: 7, view: \"short\")\n  }\n}\n",
            ),
            (
                "client/tests/OrderTestSetup.swift",
                "@TestSetup\nfinal class OrderTestSetup {\n    static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters {\n        var adapters = BehaviorAdapters()\n        adapters.http[\"orders\"] = BehaviorHTTPAdapter { request, _ in\n            .object([\"path\": request.get(\"path\"), \"view\": request.get(\"query\").get(\"view\")])\n        }\n        return adapters\n    }\n}\n",
            ),
            (
                "client/tests/OrderServiceTest.swift",
                "@Test(OrderTestSetup, {\"partition\":\"tests\"})\nfinal class OrderServiceTest {\n    @Fake\n    var ordersClient: OrdersClient? = nil\n\n    func testGetFillsThePathVariableAndTheRequestParam() async throws {\n        let service: OrderService = try construct()\n        let echo = try await service.read()\n        try Assert.equals(\"/api/orders/7\", echo.path)\n        try Assert.equals(\"short\", echo.view)\n    }\n}\n",
            ),
        ],
    };

    const GO: Self = Self {
        language: "Go",
        program: "go",
        version_flag: "version",
        unit: "client/tests/OrderServiceTest.go",
        files: &[
            (
                "client/models/OrderEcho.go",
                "type OrderEcho struct {\n\tPath string `json:\"path\"`\n\tView string `json:\"view\"`\n}\n",
            ),
            (
                "client/remotes/OrdersClient.go",
                "// @HTTPClient(\"/api\")\ntype OrdersClient interface {\n\t// @GetRequest(\"/orders/{id}\")\n\tGetOrder(\n\t// @PathVariable\n\tid int,\n\t// @RequestParam(\"view\")\n\tview string) (OrderEcho, error)\n}\n",
            ),
            (
                "client/setups/OrdersSetup.go",
                "// @Setup\ntype OrdersSetup struct{}\n\nfunc (OrdersSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {\n\treturn NewBehaviorConfiguration(), nil\n}\n",
            ),
            (
                "client/services/OrderService.go",
                "// @Service\ntype OrderService struct {\n\t// @Needs\n\tordersClient *OrdersClient\n}\n\nfunc (service *OrderService) Read() (OrderEcho, error) {\n\treturn service.ordersClient.GetOrder(7, \"short\")\n}\n",
            ),
            (
                "client/tests/OrderTestSetup.go",
                "// @TestSetup\ntype OrderTestSetup struct{}\n\nfunc (OrderTestSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {\n\tconfig := NewBehaviorConfiguration()\n\tconfig.Http[\"orders\"] = OrderTestSetup{}\n\treturn config, nil\n}\n\nfunc (OrderTestSetup) Exchange(request *BehaviorHttpRequest, execution *BehaviorExecution) (BehaviorValue, error) {\n\treturn map[string]any{\"path\": request.Path, \"view\": request.Query[\"view\"]}, nil\n}\n",
            ),
            (
                "client/tests/OrderServiceTest.go",
                "// @Test(OrderTestSetup, {\"partition\":\"tests\"})\ntype OrderServiceTest struct {\n\tbehaviors *BehaviorContext\n\t// @Fake\n\tordersClient *OrdersClient\n}\n\nfunc (test *OrderServiceTest) TestGetFillsThePathVariableAndTheRequestParam() {\n\tservice := &OrderService{}\n\ttest.construct(service)\n\techo, err := service.Read()\n\tAssert.Equals(nil, err)\n\tAssert.Equals(\"/api/orders/7\", echo.Path)\n\tAssert.Equals(\"short\", echo.View)\n}\n",
            ),
        ],
    };

    const RUBY: Self = Self {
        language: "Ruby",
        program: "ruby",
        version_flag: "--version",
        unit: "client/tests/OrderServiceTest.rb",
        files: &[
            (
                "client/models/OrderEcho.rb",
                "# @!attribute [r] path\n#   @return [String]\n# @!attribute [r] view\n#   @return [String]\nOrderEcho = Data.define(:path, :view)\n",
            ),
            (
                "client/remotes/OrdersClient.rb",
                "#[HTTPClient(\"/api\")]\nclass OrdersClient\n  # @param id [Integer]\n  # @param view [String]\n  # @return [OrderEcho]\n  #[GetRequest(\"/orders/{id}\")]\n  def get_order(\n    #[PathVariable]\n    id,\n    #[RequestParam(\"view\")]\n    view\n  )\n  end\nend\n",
            ),
            (
                "client/services/OrderService.rb",
                "#[Service]\nclass OrderService\n  #[Needs]\n  # @return [OrdersClient]\n  attr_reader :orders_client\n\n  # @return [OrderEcho]\n  def read\n    orders_client.get_order(7, \"short\")\n  end\nend\n",
            ),
            (
                "client/tests/OrderTestSetup.rb",
                "#[TestSetup]\nclass OrderTestSetup\n  # @param execution [BehaviorExecution]\n  # @return [Hash]\n  def self.configure(execution)\n    { http: { orders: { exchange: method(:exchange) } } }\n  end\n\n  # @param request [Hash]\n  # @param execution [BehaviorExecution]\n  # @return [Object]\n  def self.exchange(request, execution)\n    { \"path\" => request[\"path\"], \"view\" => request[\"query\"][\"view\"] }\n  end\nend\n",
            ),
            (
                "client/tests/OrderServiceTest.rb",
                "#[Test(OrderTestSetup, {\"partition\":\"ruby-request-parts\"})]\nclass OrderServiceTest\n  #[Fake]\n  attr_accessor :orders_client\n\n  # @return [void]\n  def test_get_fills_the_path_variable_and_the_request_param\n    echo = construct(OrderService).read\n    Assert.equals(\"/api/orders/7\", echo.path)\n    Assert.equals(\"short\", echo.view)\n  end\nend\n",
            ),
        ],
    };

    const CPP: Self = Self {
        language: "C++",
        program: "clang++",
        version_flag: "--version",
        unit: "client/tests/OrderServiceTest.hpp",
        files: &[
            (
                "client/models/OrderEcho.hpp",
                "#pragma once\n#include <string>\n\nstruct OrderEcho {\n    std::string uri;\n};\n",
            ),
            (
                "client/remotes/OrdersClient.hpp",
                "#pragma once\n#include <string>\n#include \"../models/OrderEcho.hpp\"\n\nclass [[HTTPClient(\"/api\")]] OrdersClient {\npublic:\n    [[GetRequest(\"/orders/{id}\")]]\n    Task<OrderEcho> getOrder([[PathVariable]] int id, [[RequestParam(\"view\")]] std::string view);\n};\n",
            ),
            (
                "client/services/OrderService.hpp",
                "#pragma once\n#include <string>\n#include \"../remotes/OrdersClient.hpp\"\n\nclass [[Service]] OrderService {\npublic:\n    std::string uri;\n    [[Needs]] OrdersClient& ordersClient;\n\n    Task<void> read() {\n        OrderEcho echo = co_await ordersClient.getOrder(7, \"short\");\n        uri = echo.uri;\n    }\n};\n",
            ),
            (
                "client/tests/OrderTestSetup.hpp",
                "#pragma once\n#include \"../models/OrderEcho.hpp\"\n\nclass [[TestSetup]] OrderTestSetup {\npublic:\n    static BehaviorAdapters configure() {\n        BehaviorAdapters adapters;\n        adapters.http[\"orders\"] = [](const tachyon::http::HttpRequestMessage& request) {\n            return tachyon::json::JsonSerializer::serializeToNode(OrderEcho{request.requestUri});\n        };\n        return adapters;\n    }\n};\n",
            ),
            (
                "client/tests/OrderServiceTest.hpp",
                "#pragma once\n#include \"../services/OrderService.hpp\"\n#include \"../remotes/OrdersClient.hpp\"\n\nclass [[Test(OrderTestSetup)]] OrderServiceTest {\npublic:\n    [[Fake]] OrdersClient ordersClient;\n\n    Task<void> testGetFillsThePathVariableAndTheRequestParam() {\n        OrderService service = construct<OrderService>();\n        co_await service.read();\n        Assert::equals(\"/api/orders/7?view=short\", service.uri);\n    }\n};\n",
            ),
        ],
    };

    fn available(&self) -> bool {
        crate::kotlin_home::KotlinHome::command(self.program)
            .arg(self.version_flag)
            .output()
            .is_ok_and(|output| output.status.success())
    }

    fn write(&self, root: &Path) {
        for (relative, source) in self.files {
            let path = root.join(relative);
            std::fs::create_dir_all(path.parent().expect("fixture parent"))
                .expect("fixture directory");
            std::fs::write(path, source).expect("fixture source");
        }
    }

    fn check(&self) {
        if !self.available() {
            eprintln!(
                "skipping {}: {} is unavailable",
                self.language, self.program
            );
            return;
        }
        let root = tempfile::tempdir().expect("test project");
        self.write(root.path());
        let report = super::run(root.path(), Path::new(self.unit))
            .unwrap_or_else(|failure| panic!("{}: {failure}", self.language));
        let outcomes: Vec<(String, bool)> = report
            .cases
            .iter()
            .map(|case| (case.name.clone(), case.passed))
            .collect();
        assert_eq!(outcomes.len(), 1, "{}: {report:?}", self.language);
        assert!(outcomes[0].1, "{}: {report:?}", self.language);
    }
}

#[test]
fn a_dart_http_client_fills_its_path_variable_and_request_param() {
    RequestPartsProject::DART.check();
}

#[test]
fn a_rust_http_client_fills_its_path_variable_and_request_param() {
    RequestPartsProject::RUST.check();
}

#[test]
fn a_csharp_http_client_fills_its_path_variable_and_request_param() {
    RequestPartsProject::CSHARP.check();
}

#[test]
fn a_kotlin_http_client_fills_its_path_variable_and_request_param() {
    RequestPartsProject::KOTLIN.check();
}

#[test]
fn a_swift_http_client_fills_its_path_variable_and_request_param() {
    RequestPartsProject::SWIFT.check();
}

#[test]
fn a_go_http_client_fills_its_path_variable_and_request_param() {
    RequestPartsProject::GO.check();
}

#[test]
fn a_ruby_http_client_fills_its_path_variable_and_request_param() {
    RequestPartsProject::RUBY.check();
}

#[test]
fn a_cpp_http_client_fills_its_path_variable_and_request_param() {
    RequestPartsProject::CPP.check();
}
