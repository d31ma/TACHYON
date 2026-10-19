#![allow(clippy::expect_used)]

use std::path::Path;

struct SwiftConstructProject;

impl SwiftConstructProject {
    const FILES: [(&'static str, &'static str); 6] = [
        (
            "client/models/Greeting.swift",
            "struct Greeting: Codable {\n    let message: String\n}\n",
        ),
        (
            "client/remotes/GreetingClient.swift",
            "@HTTPClient(\"/api\")\nprotocol GreetingClient {\n    @GetRequest(\"/greeting\")\n    func getGreeting() async throws -> Greeting\n\n    @GetRequest(\"/greetings\")\n    func greetings() async throws -> [Greeting]\n}\n",
        ),
        (
            "client/setups/GreetingSetup.swift",
            "@Setup\nfinal class GreetingSetup {\n    static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters {\n        BehaviorAdapters()\n    }\n}\n",
        ),
        (
            "client/services/CounterService.swift",
            "@Service\nfinal class CounterService {\n    @Needs\n    private let greetingClient: GreetingClient\n    var count: Int = 0\n    var message: String = \"\"\n\n    func increment() {\n        count += 1\n    }\n\n    func askServer() async throws {\n        message = try await greetingClient.getGreeting().message\n    }\n\n    func countGreetings() async throws {\n        count = try await greetingClient.greetings().count\n    }\n}\n",
        ),
        (
            "client/tests/HelloTestSetup.swift",
            "@TestSetup\nfinal class HelloTestSetup {\n    static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters {\n        var adapters = BehaviorAdapters()\n        adapters.http[\"greeting\"] = BehaviorHTTPAdapter { _, _ in\n            .object([\"message\": .string(\"Hi\")])\n        }\n        return adapters\n    }\n}\n",
        ),
        (
            "client/tests/CounterServiceTest.swift",
            "@Test(HelloTestSetup, {\"partition\":\"tests\"})\nfinal class CounterServiceTest {\n    @Fake\n    var greetingClient: GreetingClient? = nil\n\n    func testIncrementAddsOne() async throws {\n        let counter: CounterService = try construct()\n        counter.increment()\n        try Assert.equals(1, counter.count)\n    }\n\n    func testAskServerSavesTheTypedMessage() async throws {\n        let counter: CounterService = try construct()\n        try await counter.askServer()\n        try Assert.equals(\"Hi\", counter.message)\n    }\n\n    func testAnAnswerOfAnotherShapeIsRefused() async throws {\n        let counter: CounterService = try construct()\n        var code = \"\"\n        do { try await counter.countGreetings() } catch let error as BehaviorError { code = error.code }\n        try Assert.equals(\"HTTP_RESPONSE\", code)\n    }\n\n    func testAnAssignedFakeIsUsedAsIs() async throws {\n        greetingClient = GreetingClient()\n        let counter: CounterService = try construct()\n        var code = \"\"\n        do { try await counter.askServer() } catch let error as BehaviorError { code = error.code }\n        try Assert.equals(\"HTTP_MISSING\", code)\n    }\n}\n",
        ),
    ];

    fn write(root: &Path) {
        for (relative, source) in Self::FILES {
            let path = root.join(relative);
            std::fs::create_dir_all(path.parent().expect("fixture parent"))
                .expect("fixture directory");
            std::fs::write(path, source).expect("fixture source");
        }
    }
}

#[test]
fn swift_tests_construct_services_with_injected_fake_clients_and_typed_answers() {
    if std::process::Command::new("swiftc")
        .arg("--version")
        .output()
        .is_err()
    {
        assert!(
            [cfg!(unix), std::env::var_os("CI").is_some()] != [true, true],
            "CI must provide swiftc on Linux and macOS"
        );
        eprintln!("skipping: swiftc is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("test project");
    SwiftConstructProject::write(root.path());
    let report = super::run(
        root.path(),
        Path::new("client/tests/CounterServiceTest.swift"),
    )
    .expect("native Swift test unit");
    assert_eq!(report.cases.len(), 4, "{report:?}");
    assert!(report.cases.iter().all(|case| case.passed), "{report:?}");
}
