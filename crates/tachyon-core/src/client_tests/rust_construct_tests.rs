#![allow(clippy::expect_used)]

use std::path::Path;

struct RustConstructProject;

impl RustConstructProject {
    const FILES: [(&'static str, &'static str); 7] = [
        (
            "client/models/Greeting.rs",
            "#[derive(Clone, Debug, Default)]\npub struct Greeting {\n    pub message: String,\n}\n",
        ),
        (
            "client/remotes/GreetingClient.rs",
            "use crate::models::Greeting;\n\n#[HTTPClient(\"/api\")]\ntrait GreetingClient {\n    #[GetRequest(\"/greeting\")]\n    async fn get_greeting(&self) -> BehaviorResult<Greeting>;\n}\n",
        ),
        (
            "client/setups/GreetingSetup.rs",
            "#[tac::setup]\nstruct GreetingSetup {}\nimpl GreetingSetup {\n    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {\n        Ok(BehaviorConfiguration::default())\n    }\n}\n",
        ),
        (
            "client/services/CounterService.rs",
            "#[Service]\npub struct CounterService {\n    #[Needs]\n    greeting_client: std::sync::Arc<GreetingClient>,\n    pub count: i32,\n    pub message: String,\n}\n\nimpl CounterService {\n    pub fn increment(&mut self) {\n        self.count += 1;\n    }\n\n    pub async fn ask_server(&mut self) -> BehaviorResult<()> {\n        self.message = self.greeting_client.get_greeting().await?.message;\n        Ok(())\n    }\n}\n",
        ),
        (
            "client/tests/HelloTestSetup.rs",
            "#[TestSetup]\nstruct HelloTestSetup {}\nimpl HelloTestSetup {\n    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {\n        struct GreetingFake;\n        impl BehaviorHttpAdapter for GreetingFake {\n            fn exchange(&self, _request: &BehaviorHttpRequest, _execution: &BehaviorExecution) -> BehaviorResult<BehaviorValue> {\n                Ok(BehaviorValue::object([(\"message\", BehaviorValue::from(\"Hi\"))]))\n            }\n        }\n\n        let mut configuration = BehaviorConfiguration::default();\n        configuration.http.insert(\"greeting\".into(), std::sync::Arc::new(GreetingFake));\n        Ok(configuration)\n    }\n}\n",
        ),
        (
            "client/tests/CounterServiceTest.rs",
            "#[Test(HelloTestSetup, {\"partition\":\"tests\"})]\nstruct CounterServiceTest {\n    behaviors: std::cell::RefCell<Option<BehaviorContext>>,\n    #[Fake]\n    greeting_client: std::cell::RefCell<Option<std::sync::Arc<GreetingClient>>>,\n}\nimpl CounterServiceTest {\n    fn new() -> Self {\n        Self { behaviors: std::cell::RefCell::new(None), greeting_client: std::cell::RefCell::new(None) }\n    }\n\n    fn testIncrementAddsOne(&self) {\n        let mut counter: CounterService = self.construct();\n        counter.increment();\n        Assert::equals(1, counter.count);\n    }\n\n    async fn testAskServerSavesTheTypedMessage(&self) {\n        let mut counter: CounterService = self.construct();\n        let answer = counter.ask_server().await;\n        Assert::is_true(answer.is_ok());\n        Assert::equals(\"Hi\", counter.message);\n    }\n\n    async fn testAnAssignedFakeIsUsedAsIs(&self) {\n        self.greeting_client.replace(Some(std::sync::Arc::new(GreetingClient::new())));\n        let mut counter: CounterService = self.construct();\n        let answer = counter.ask_server().await;\n        Assert::equals(Some(\"HTTP_MISSING\"), answer.err().map(|error| error.code));\n        Assert::equals(\"\", counter.message);\n    }\n}\n",
        ),
        (
            "client/tests/WrongShapeTest.rs",
            "#[Test(ShapeTestSetup, {\"partition\":\"shape\"})]\nstruct WrongShapeTest {\n    behaviors: std::cell::RefCell<Option<BehaviorContext>>,\n    #[Fake]\n    greeting_client: std::cell::RefCell<Option<std::sync::Arc<GreetingClient>>>,\n}\nimpl WrongShapeTest {\n    fn new() -> Self {\n        Self { behaviors: std::cell::RefCell::new(None), greeting_client: std::cell::RefCell::new(None) }\n    }\n\n    async fn testAListIsNotAGreeting(&self) {\n        let mut counter: CounterService = self.construct();\n        let answer = counter.ask_server().await;\n        Assert::equals(Some(\"HTTP_RESPONSE\"), answer.err().map(|error| error.code));\n    }\n}\n",
        ),
    ];
    const SHAPE_SETUP: (&'static str, &'static str) = (
        "client/tests/ShapeTestSetup.rs",
        "#[TestSetup]\nstruct ShapeTestSetup {}\nimpl ShapeTestSetup {\n    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {\n        struct ListFake;\n        impl BehaviorHttpAdapter for ListFake {\n            fn exchange(&self, _request: &BehaviorHttpRequest, _execution: &BehaviorExecution) -> BehaviorResult<BehaviorValue> {\n                Ok(BehaviorValue::Array(Vec::new()))\n            }\n        }\n\n        let mut configuration = BehaviorConfiguration::default();\n        configuration.http.insert(\"greeting\".into(), std::sync::Arc::new(ListFake));\n        Ok(configuration)\n    }\n}\n",
    );

    fn write(root: &Path) {
        for (relative, source) in Self::FILES.into_iter().chain([Self::SHAPE_SETUP]) {
            let path = root.join(relative);
            std::fs::create_dir_all(path.parent().expect("fixture parent"))
                .expect("fixture directory");
            std::fs::write(path, source).expect("fixture source");
        }
    }
}

#[test]
fn rust_tests_construct_services_with_injected_fake_clients_and_await_typed_answers() {
    let root = tempfile::tempdir().expect("test project");
    RustConstructProject::write(root.path());
    for (name, cases) in [("CounterServiceTest", 3), ("WrongShapeTest", 1)] {
        let relative = format!("client/tests/{name}.rs");
        let report = super::run(root.path(), Path::new(&relative)).expect("native Rust test unit");
        assert_eq!(report.cases.len(), cases, "{name}");
        assert!(report.cases.iter().all(|case| case.passed), "{report:?}");
    }
}
