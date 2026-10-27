#![allow(clippy::expect_used)]

use std::time::Duration;
use tachyon_contracts::{HandlerRequest, HttpMethod};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
};

struct DescribedService {
    extension: &'static str,
    compiler: &'static str,
    version_flag: &'static str,
    files: [(&'static str, &'static str); 4],
    reads_registration: bool,
}

impl DescribedService {
    const DESCRIPTION: &'static str = "Counts the attempts of every order";

    const JAVA: Self = Self {
        extension: "java",
        compiler: "javac",
        version_flag: "-version",
        files: [
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
        reads_registration: false,
    };

    const GROOVY: Self = Self {
        extension: "groovy",
        compiler: "groovyc",
        version_flag: "--version",
        files: [
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
        reads_registration: false,
    };

    const KOTLIN: Self = Self {
        extension: "kt",
        compiler: "kotlinc",
        version_flag: "-version",
        files: [
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
        reads_registration: false,
    };

    const DART: Self = Self {
        extension: "dart",
        compiler: "dart",
        version_flag: "--version",
        files: [
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
        reads_registration: true,
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

    async fn verify(&self) {
        if !self.available() {
            return;
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
        if self.reads_registration {
            assert_eq!(
                body["description"],
                Self::DESCRIPTION,
                "{}: {body}",
                self.extension
            );
        } else {
            assert_eq!(
                body["again"],
                serde_json::json!({"id": "A-2", "attempts": 4}),
                "{}: {body}",
                self.extension
            );
        }
        self.refuses_a_malformed_description();
    }
}

#[tokio::test]
async fn a_described_java_service_keeps_its_behavior() {
    DescribedService::JAVA.verify().await;
}

#[tokio::test]
async fn a_described_groovy_service_keeps_its_behavior() {
    DescribedService::GROOVY.verify().await;
}

#[tokio::test]
async fn a_described_kotlin_service_keeps_its_behavior() {
    DescribedService::KOTLIN.verify().await;
}

#[tokio::test]
async fn a_described_dart_service_carries_its_description_into_the_registration() {
    DescribedService::DART.verify().await;
}
