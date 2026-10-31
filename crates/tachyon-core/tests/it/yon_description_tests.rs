#![allow(clippy::expect_used)]

use crate::support::one_test_per_case::one_test_per_case;
use std::time::Duration;
use tachyon_contracts::{HandlerRequest, HttpMethod};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
};

struct DescribedService {
    extension: &'static str,
    compiler: &'static str,
    version_flag: &'static str,
    files: &'static [(&'static str, &'static str)],
}

impl DescribedService {
    const DESCRIPTION: &'static str = "Counts the attempts of every order";

    const KEEPING_BEHAVIOR: [Self; 6] = [
        Self::JAVA,
        Self::GROOVY,
        Self::KOTLIN,
        Self::RUBY,
        Self::RUST,
        Self::GO,
    ];

    const JAVA: Self = Self {
        extension: "java",
        compiler: "javac",
        version_flag: "-version",
        files: &[
            (
                "server/setups/OrdersSetup.java",
                "@Setup\nclass OrdersSetup {\n    static Behaviors.Configuration configure(Behaviors.Execution execution) {\n        return new Behaviors.Configuration(java.util.Map.of(), java.util.Map.of(), java.util.Map.of(),\n            (call, current) -> true);\n    }\n}\n",
            ),
            (
                "server/services/OrdersService.java",
                "@Service\n@Description(\"Counts the attempts of every order\")\nclass OrdersService {\n    Behaviors.Context behaviors;\n    private int attempts = 0;\n\n    @OnStart\n    void open() {\n        Behaviors.Options options = new Behaviors.Options(\"orders\", new Behaviors.Signal());\n        behaviors = Behaviors.create(\"OrdersSetup\", options);\n    }\n\n    @Retry({\"maxRetries\":3,\"delayMs\":10,\"multiplier\":2,\"maxDelayMs\":200,\"jitterMs\":5})\n    Object submit(String id) {\n        attempts++;\n        if (attempts < 3) throw new IllegalStateException(\"the payment gateway is unavailable\");\n        return java.util.Map.of(\"id\", id, \"attempts\", attempts);\n    }\n}\n",
            ),
            (
                "server/models/OrderReport.java",
                "record OrderReport(Object submitted, Object again) {}\n",
            ),
            (
                "server/controllers/OrdersController.java",
                "import models.OrderReport;\n\n@RequestRoute(\"/orders\")\n@Public\nclass OrdersController {\n    @Needs\n    final OrdersService orders;\n\n    @GetRoute\n    OrderReport getOrders() {\n        Object submitted = orders.submit(\"A-1\");\n        return new OrderReport(submitted, orders.submit(\"A-2\"));\n    }\n}\n",
            ),
        ],
    };

    const GROOVY: Self = Self {
        extension: "groovy",
        compiler: "groovyc",
        version_flag: "--version",
        files: &[
            (
                "server/setups/OrdersSetup.groovy",
                "@Setup\nclass OrdersSetup {\n    static Behaviors.Configuration configure(Behaviors.Execution execution) {\n        return new Behaviors.Configuration(java.util.Map.of(), java.util.Map.of(), java.util.Map.of(),\n            (call, current) -> true)\n    }\n}\n",
            ),
            (
                "server/services/OrdersService.groovy",
                "@Service\n@Description(\"Counts the attempts of every order\")\nclass OrdersService {\n    Behaviors.Context behaviors\n    private int attempts = 0\n\n    @OnStart\n    void open() {\n        Behaviors.Options options = new Behaviors.Options(\"orders\", new Behaviors.Signal())\n        behaviors = Behaviors.create(\"OrdersSetup\", options)\n    }\n\n    @Retry({\"maxRetries\":3,\"delayMs\":10,\"multiplier\":2,\"maxDelayMs\":200,\"jitterMs\":5})\n    Object submit(String id) {\n        attempts++\n        if (attempts < 3) throw new IllegalStateException(\"the payment gateway is unavailable\")\n        return java.util.Map.of(\"id\", id, \"attempts\", attempts)\n    }\n}\n",
            ),
            (
                "server/models/OrderReport.groovy",
                "record OrderReport(Object submitted, Object again) {}\n",
            ),
            (
                "server/controllers/OrdersController.groovy",
                "import models.OrderReport\n\n@RequestRoute(\"/orders\")\n@Public\nclass OrdersController {\n    @Needs\n    final OrdersService orders\n\n    @GetRoute\n    OrderReport getOrders() {\n        Object submitted = orders.submit(\"A-1\")\n        return new OrderReport(submitted, orders.submit(\"A-2\"))\n    }\n}\n",
            ),
        ],
    };

    const KOTLIN: Self = Self {
        extension: "kt",
        compiler: "kotlinc",
        version_flag: "-version",
        files: &[
            (
                "server/setups/OrdersSetup.kt",
                "@Setup\nclass OrdersSetup {\n    companion object {\n        fun configure(execution: Behaviors.Execution): Behaviors.Configuration =\n            Behaviors.Configuration(caches = mapOf(), authorize = Behaviors.Authorization { _, _ -> true })\n    }\n}\n",
            ),
            (
                "server/services/OrdersService.kt",
                "@Service\n@Description(\"Counts the attempts of every order\")\nclass OrdersService {\n    lateinit var behaviors: Behaviors.Context\n    private var attempts: Int = 0\n\n    @OnStart\n    fun open() {\n        val options = Behaviors.Options(\"orders\", Behaviors.Signal())\n        behaviors = Behaviors.create(\"OrdersSetup\", options)\n    }\n\n    @Retry({\"maxRetries\":3,\"delayMs\":10,\"multiplier\":2,\"maxDelayMs\":200,\"jitterMs\":5})\n    fun submit(id: String): Any? {\n        attempts++\n        if (attempts < 3) throw IllegalStateException(\"the payment gateway is unavailable\")\n        return mapOf(\"id\" to id, \"attempts\" to attempts)\n    }\n}\n",
            ),
            (
                "server/models/OrderReport.kt",
                "data class OrderReport(val submitted: Any?, val again: Any?)\n",
            ),
            (
                "server/controllers/OrdersController.kt",
                "import models.OrderReport\n\n@RequestRoute(\"/orders\")\n@Public\nclass OrdersController {\n    @Needs\n    val orders: OrdersService\n\n    @GetRoute\n    fun getOrders(): OrderReport {\n        val submitted = orders.submit(\"A-1\")\n        return OrderReport(submitted, orders.submit(\"A-2\"))\n    }\n}\n",
            ),
        ],
    };

    const DART: Self = Self {
        extension: "dart",
        compiler: "dart",
        version_flag: "--version",
        files: &[
            (
                "server/setups/OrdersSetup.dart",
                "@Setup\nclass OrdersSetup {\n  static BehaviorAdapters configure(BehaviorExecution execution) {\n    return BehaviorAdapters(authorize: (call, execution) async => true);\n  }\n}\n",
            ),
            (
                "server/services/OrdersService.dart",
                "@Service\n@Description(\"Counts the attempts of every order\")\nclass OrdersService {\n  late BehaviorContext behaviors;\n  int _attempts = 0;\n\n  @OnStart\n  void open() {\n    behaviors = Behaviors.create(OrdersSetup, partition: 'orders', signal: BehaviorSignal());\n  }\n\n  @Retry({\"maxRetries\":3,\"delayMs\":10,\"multiplier\":2,\"maxDelayMs\":200,\"jitterMs\":5})\n  Future<Object?> submit(String id) async {\n    _attempts += 1;\n    if (_attempts < 3) throw StateError('the payment gateway is unavailable');\n    return {'id': id, 'attempts': _attempts};\n  }\n\n  String describe() {\n    return BehaviorJson.get(Behaviors._typePolicies(OrdersService), 'description') as String;\n  }\n}\n",
            ),
            (
                "server/models/OrderReport.dart",
                "class OrderReport {\n  final Object? submitted;\n  final String description;\n\n  const OrderReport(this.submitted, this.description);\n}\n",
            ),
            (
                "server/controllers/OrdersController.dart",
                "import 'models/OrderReport.dart';\n\n@RequestRoute(\"/orders\")\n@Public\nclass OrdersController {\n  @Needs\n  final OrdersService orders;\n\n  @GetRoute\n  Future<OrderReport> getOrders() async {\n    final submitted = await orders.submit('A-1');\n    return OrderReport(submitted, orders.describe());\n  }\n}\n",
            ),
        ],
    };

    const RUBY: Self = Self {
        extension: "rb",
        compiler: "ruby",
        version_flag: "--version",
        files: &[
            (
                "server/setups/OrdersSetup.rb",
                "#[Setup]\nclass OrdersSetup\n  def self.configure(execution)\n    { authorize: ->(call, current) { true } }\n  end\nend\n",
            ),
            (
                "server/services/OrdersService.rb",
                "#[Service]\n#[Description(\"Counts the attempts of every order\")]\nclass OrdersService\n  attr_accessor :behaviors\n\n  # @return [void]\n  #[OnStart]\n  def open\n    @attempts = 0\n    @behaviors = Behaviors.create(OrdersSetup, partition: \"orders\", signal: BehaviorSignal.new)\n  end\n\n  # @param id [String]\n  # @return [Hash]\n  #[Retry({\"maxRetries\":3,\"delayMs\":10,\"multiplier\":2,\"maxDelayMs\":200,\"jitterMs\":5})]\n  def submit(id)\n    @attempts += 1\n    raise \"the payment gateway is unavailable\" if @attempts < 3\n    { id: id, attempts: @attempts }\n  end\nend\n",
            ),
            (
                "server/models/OrderReport.rb",
                "# @!attribute [r] submitted\n#   @return [Hash]\n# @!attribute [r] again\n#   @return [Hash]\nOrderReport = Data.define(:submitted, :again)\n",
            ),
            (
                "server/controllers/OrdersController.rb",
                "#[RequestRoute(\"/orders\")]\n#[Public]\nclass OrdersController\n  #[Needs]\n  # @return [OrdersService]\n  attr_reader :orders_service\n\n  # @return [OrderReport]\n  #[GetRoute]\n  def get_orders\n    submitted = orders_service.submit(\"A-1\")\n    OrderReport.new(submitted: submitted, again: orders_service.submit(\"A-2\"))\n  end\nend\n",
            ),
        ],
    };

    const RUST: Self = Self {
        extension: "rs",
        compiler: "cargo",
        version_flag: "--version",
        files: &[
            (
                "server/setups/OrdersSetup.rs",
                "#[Setup]\nstruct OrdersSetup {}\n\nimpl OrdersSetup {\n    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {\n        Ok(BehaviorConfiguration::default())\n    }\n}\n",
            ),
            (
                "server/services/OrdersService.rs",
                "#[Service]\n#[Description(\"Counts the attempts of every order\")]\nstruct OrdersService {\n    behaviors: BehaviorContext,\n    attempts: std::sync::atomic::AtomicI64,\n}\n\nimpl OrdersService {\n    fn new() -> Self {\n        let options = BehaviorOptions::new(\"orders\", BehaviorSignal::new());\n        let behaviors = Behaviors::create::<OrdersSetup>(options).expect(\"behaviors\");\n        Self { behaviors, attempts: std::sync::atomic::AtomicI64::new(0) }\n    }\n\n    #[Retry({\"maxRetries\":3,\"delayMs\":10,\"multiplier\":2,\"maxDelayMs\":200,\"jitterMs\":5})]\n    fn submit(&self, id: BehaviorValue) -> BehaviorResult<BehaviorValue> {\n        let attempts = self.attempts.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;\n        if attempts < 3 {\n            return Err(BehaviorError::new(\"adapter\", \"the payment gateway is unavailable\"));\n        }\n        Ok(BehaviorValue::object([(\"id\", id), (\"attempts\", BehaviorValue::Number(attempts as f64))]))\n    }\n}\n",
            ),
            (
                "server/controllers/OrdersController.rs",
                "#[RequestRoute(\"/orders\")]\n#[Public]\nstruct OrdersController {\n    #[Needs]\n    orders_service: std::sync::Arc<OrdersService>,\n}\n\nimpl OrdersController {\n    #[GetRoute]\n    fn get_orders(&self) -> http::Response<Vec<u8>> {\n        let submitted = self.orders_service.submit(BehaviorValue::from(\"A-1\")).expect(\"submitted\");\n        let again = self.orders_service.submit(BehaviorValue::from(\"A-2\")).expect(\"again\");\n        let report = BehaviorValue::object([(\"submitted\", submitted), (\"again\", again)]);\n        http::Response::builder()\n            .header(\"content-type\", \"application/json\")\n            .body(report.to_json(1_048_576).expect(\"JSON\").into_bytes())\n            .unwrap_or_default()\n    }\n}\n",
            ),
        ],
    };

    const GO: Self = Self {
        extension: "go",
        compiler: "go",
        version_flag: "version",
        files: &[
            (
                "server/setups/OrdersSetup.go",
                "// @Setup\ntype OrdersSetup struct{}\n\nfunc (OrdersSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {\n\treturn &BehaviorConfiguration{}, nil\n}\n",
            ),
            (
                "server/services/OrdersService.go",
                "// @Service\n// @Description(\"Counts the attempts of every order\")\ntype OrdersService struct {\n\tbehaviors *BehaviorContext\n\tattempts  int\n}\n\n// @OnStart\nfunc (orders *OrdersService) Open() error {\n\tcontext, err := BehaviorsCreate[OrdersSetup](NewBehaviorOptions(\"orders\", NewBehaviorSignal()))\n\torders.behaviors = context\n\treturn err\n}\n\n// @Retry({\"maxRetries\":3,\"delayMs\":10,\"multiplier\":2,\"maxDelayMs\":200,\"jitterMs\":5})\nfunc (orders *OrdersService) Submit(id BehaviorValue) (BehaviorValue, error) {\n\torders.attempts++\n\tif orders.attempts < 3 {\n\t\treturn nil, &BehaviorError{Code: \"adapter\", Message: \"the payment gateway is unavailable\"}\n\t}\n\treturn map[string]any{\"id\": id, \"attempts\": orders.attempts}, nil\n}\n",
            ),
            (
                "server/controllers/OrdersController.go",
                "// @RequestRoute(\"/orders\")\n// @Public\ntype OrdersController struct {\n\t// @Needs\n\tordersService *OrdersService\n}\n\n// @GetRoute\nfunc (controller *OrdersController) GetOrders() any {\n\tsubmitted, err := controller.ordersService.Submit(\"A-1\")\n\tif err != nil {\n\t\treturn err.Error()\n\t}\n\tagain, err := controller.ordersService.Submit(\"A-2\")\n\tif err != nil {\n\t\treturn err.Error()\n\t}\n\treturn map[string]any{\"submitted\": submitted, \"again\": again}\n}\n",
            ),
        ],
    };

    fn available(&self) -> bool {
        let found =
            crate::support::program_probe::ProgramProbe::answers(self.compiler, self.version_flag);
        if !crate::support::toolchain::Toolchain::checked(self.compiler, found) {
            eprintln!("skipping: {} is unavailable", self.compiler);
            return false;
        }
        true
    }

    fn entry(&self) -> String {
        format!("server/controllers/OrdersController.{}", self.extension)
    }

    fn project(&self, description: &str) -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        for (relative, source) in self.files {
            let target = root.path().join(relative);
            std::fs::create_dir_all(target.parent().expect("parent")).expect("folders");
            let source = source.replace(Self::DESCRIPTION, description);
            std::fs::write(target, source).expect("source");
        }
        root
    }

    fn refuses_a_malformed_description(&self) {
        let root = self.project("Counts \\n orders");
        let failure = HandlerSource::discover(root.path(), self.entry())
            .expect_err("a description with an escape is refused");
        assert!(
            failure
                .to_string()
                .contains("Description requires one quoted line"),
            "{}: {failure}",
            self.extension
        );
    }

    fn keeping_behavior(extension: &str) -> Self {
        Self::KEEPING_BEHAVIOR
            .into_iter()
            .find(|service| service.extension == extension)
            .expect("described service")
    }

    async fn answer(&self) -> Option<serde_json::Value> {
        if !self.available() {
            return None;
        }
        let root = self.project(Self::DESCRIPTION);
        let entry = self.entry();
        let source = HandlerSource::discover(root.path(), &entry)
            .unwrap_or_else(|failure| panic!("{} discovery: {failure}", self.extension));
        let supervisor = HandlerSupervisor::new(HandlerSupervisorOptions {
            default_timeout: Duration::from_mins(2),
            ..HandlerSupervisorOptions::default()
        })
        .expect("supervisor");
        let response = supervisor
            .invoke(
                &source,
                &HandlerRequest::route("described", "/orders", HttpMethod::Get),
                &HandlerCancellation::default(),
            )
            .await
            .unwrap_or_else(|error| panic!("{} invocation: {error:?}", self.extension));
        assert!(
            response.error.is_none(),
            "{}: {:?}",
            self.extension,
            response.error
        );
        assert_eq!(response.status, 200, "{}", self.extension);
        let body: serde_json::Value =
            serde_json::from_str(&response.body.expect("body").data).expect("JSON body");
        assert_eq!(
            body["submitted"],
            serde_json::json!({"id": "A-1", "attempts": 3}),
            "{}: {body}",
            self.extension
        );
        Some(body)
    }

    async fn keeps_its_behavior(&self) {
        let Some(body) = self.answer().await else {
            return;
        };
        assert_eq!(
            body["again"],
            serde_json::json!({"id": "A-2", "attempts": 4}),
            "{}: {body}",
            self.extension
        );
        self.refuses_a_malformed_description();
    }

    async fn carries_its_description_into_the_registration(&self) {
        let Some(body) = self.answer().await else {
            return;
        };
        assert_eq!(
            body["description"],
            Self::DESCRIPTION,
            "{}: {body}",
            self.extension
        );
        self.refuses_a_malformed_description();
    }
}

async fn keeps_its_behavior(extension: &'static str) {
    DescribedService::keeping_behavior(extension)
        .keeps_its_behavior()
        .await;
}

one_test_per_case!(
    a_described_service_keeps_its_behavior,
    async keeps_its_behavior,
    all DescribedService::KEEPING_BEHAVIOR.map(|service| service.extension),
    [
        java => "java",
        groovy => "groovy",
        kotlin => "kt",
        ruby => "rb",
        rust => "rs",
        go => "go",
    ]
);

#[tokio::test]
async fn a_described_dart_service_carries_its_description_into_the_registration() {
    DescribedService::DART
        .carries_its_description_into_the_registration()
        .await;
}
