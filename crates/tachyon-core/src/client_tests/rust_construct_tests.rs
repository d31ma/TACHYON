#![allow(clippy::expect_used)]

use std::path::Path;

struct RustConstructProject;

impl RustConstructProject {
    const FILES: [(&'static str, &'static str); 7] = [
        (
            "client/models/Greeting.rs",
            include_str!(
                "../../../../website/content/docs/first-app/call-the-server/client/models/Greeting.rs"
            ),
        ),
        (
            "client/remotes/GreetingClient.rs",
            include_str!(
                "../../../../website/content/docs/first-app/call-the-server/client/remotes/GreetingClient.rs"
            ),
        ),
        (
            "client/setups/GreetingSetup.rs",
            include_str!(
                "../../../../website/content/docs/first-app/call-the-server/client/setups/GreetingSetup.rs"
            ),
        ),
        (
            "client/services/CounterService.rs",
            include_str!(
                "../../../../website/content/docs/first-app/call-the-server/client/services/CounterService.rs"
            ),
        ),
        (
            "client/tests/HelloTestSetup.rs",
            include_str!(
                "../../../../website/content/docs/first-app/add-tests/client/tests/HelloTestSetup.rs"
            ),
        ),
        (
            "client/tests/AssignedFakeTest.rs",
            "#[Test(HelloTestSetup, {\"partition\":\"tests\"})]\nstruct AssignedFakeTest {\n    behaviors: std::cell::RefCell<Option<BehaviorContext>>,\n    #[Fake]\n    greeting_client: std::cell::RefCell<Option<std::sync::Arc<GreetingClient>>>,\n}\nimpl AssignedFakeTest {\n    fn new() -> Self {\n        Self { behaviors: std::cell::RefCell::new(None), greeting_client: std::cell::RefCell::new(None) }\n    }\n\n    async fn testAnAssignedFakeIsUsedAsIs(&self) {\n        self.greeting_client.replace(Some(std::sync::Arc::new(GreetingClient::new())));\n        let mut counter: CounterService = self.construct();\n        let answer = counter.ask_server().await;\n        Assert::equals(Some(\"HTTP_MISSING\"), answer.err().map(|error| error.code));\n        Assert::equals(\"\", counter.message);\n    }\n}\n",
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

    fn passed(root: &Path, relative: &str) {
        let report = super::run(root, Path::new(relative)).expect("native Rust test unit");
        assert_eq!(report.cases.len(), 1, "{relative}: {report:?}");
        assert!(report.cases[0].passed, "{relative}: {report:?}");
    }
}

#[test]
fn rust_tests_construct_services_with_injected_fake_clients_and_await_typed_answers() {
    let root = tempfile::tempdir().expect("test project");
    RustConstructProject::write(root.path());
    RustConstructProject::passed(root.path(), "client/tests/AssignedFakeTest.rs");
    RustConstructProject::passed(root.path(), "client/tests/WrongShapeTest.rs");
}

struct RustSocketProject;

impl RustSocketProject {
    const FILES: [(&'static str, &'static str); 5] = [
        (
            "client/tests/ChatTestSetup.rs",
            "#[TestSetup]\nstruct ChatTestSetup {}\nimpl ChatTestSetup {\n    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {\n        Ok(BehaviorConfiguration::default())\n    }\n}\n",
        ),
        (
            "client/models/ChatMessage.rs",
            "#[derive(Clone, Debug, Default)]\npub struct ChatMessage {\n    pub text: String,\n}\n",
        ),
        (
            "client/remotes/ChatClient.rs",
            "use crate::models::ChatMessage;\n\n#[HTTPClient(\"/chat\")]\ntrait ChatClient {\n    #[SocketRequest]\n    fn receive(&self, message: ChatMessage);\n}\n",
        ),
        (
            "client/services/ChatService.rs",
            "#[Service]\npub struct ChatService {\n    #[Needs]\n    chat: std::sync::Arc<ChatClient>,\n    pub said: String,\n}\n\nimpl ChatService {\n    pub async fn say(&mut self, text: String) -> BehaviorResult<()> {\n        self.chat.send(ChatMessage { text }).await\n    }\n}\n",
        ),
        (
            "client/tests/ChatServiceTest.rs",
            "#[Test(ChatTestSetup, {\"partition\":\"chat\"})]\nstruct ChatServiceTest {\n    behaviors: std::cell::RefCell<Option<BehaviorContext>>,\n    #[Fake]\n    chat: std::cell::RefCell<Option<std::sync::Arc<ChatClient>>>,\n}\nimpl ChatServiceTest {\n    fn new() -> Self {\n        Self { behaviors: std::cell::RefCell::new(None), chat: std::cell::RefCell::new(None) }\n    }\n\n    async fn testATestHasNoPageToSendOn(&self) {\n        self.chat.replace(Some(std::sync::Arc::new(ChatClient::new())));\n        let mut chat: ChatService = self.construct();\n        let answer = chat.say(String::from(\"hello\")).await;\n        Assert::equals(Some(\"HTTP_MISSING\"), answer.err().map(|error| error.code));\n    }\n}\n",
        ),
    ];
}

#[test]
fn a_rust_socket_client_sends_nothing_in_a_test_and_answers_http_missing() {
    let root = tempfile::tempdir().expect("test project");
    for (relative, source) in RustSocketProject::FILES {
        let path = root.path().join(relative);
        std::fs::create_dir_all(path.parent().expect("fixture parent")).expect("fixture directory");
        std::fs::write(path, source).expect("fixture source");
    }
    let report = super::run(root.path(), Path::new("client/tests/ChatServiceTest.rs"))
        .expect("native Rust test unit");
    assert_eq!(report.cases.len(), 1);
    assert!(report.cases.iter().all(|case| case.passed), "{report:?}");
}
