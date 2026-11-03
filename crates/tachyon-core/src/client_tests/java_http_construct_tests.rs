#![allow(clippy::expect_used)]

use std::path::Path;

struct JavaHttpConstructProject;

impl JavaHttpConstructProject {
    const FILES: [(&'static str, &'static str); 6] = [
        (
            "client/models/Greeting.java",
            "record Greeting(String message) {}\n",
        ),
        (
            "client/remotes/GreetingClient.java",
            "@HTTPClient({\"path\":\"/api\",\"setup\":\"GreetingSetup\"})\n@CacheSettings({\"name\":\"greetings\",\"ttlMs\":60000,\"maxBytes\":4096,\"maxEntries\":8})\ninterface GreetingClient {\n    @Cached\n    @GetRequest(\"/greeting\")\n    Greeting read();\n    @GetRequest(\"/greetings\")\n    java.util.List<Greeting> all();\n}\n",
        ),
        (
            "client/setups/GreetingSetup.java",
            "@Setup\nclass GreetingSetup {\n    static Behaviors.Configuration configure(Behaviors.Execution execution) {\n        return new Behaviors.Configuration(java.util.Map.of(), java.util.Map.of(), java.util.Map.of(), (call, current) -> true);\n    }\n}\n",
        ),
        (
            "client/services/GreetingService.java",
            "@Service\nclass GreetingService {\n    @Needs\n    private final GreetingClient greetingClient;\n    String message = \"\";\n    void ask() { message = greetingClient.read().message(); }\n    int count() { return greetingClient.all().size(); }\n}\n",
        ),
        (
            "client/tests/GreetingTestSetup.java",
            "@TestSetup\nclass GreetingTestSetup {\n    static Behaviors.Configuration configure(Behaviors.Execution execution) {\n        Behaviors.HttpAdapter greeting = (request, current) -> request.get(\"path\").equals(\"/api/greetings\")\n            ? java.util.List.of(java.util.Map.of(\"message\", \"one\"), java.util.Map.of(\"message\", \"two\"))\n            : java.util.Map.of(\"message\", \"Test adapter wins\");\n        Behaviors.CacheAdapter cache = new Behaviors.CacheAdapter() {\n            private final java.util.Map<String, Behaviors.Entry> values = new java.util.HashMap<>();\n            public Behaviors.Entry get(String key, Behaviors.Execution current) {\n                return values.getOrDefault(key, new Behaviors.Entry());\n            }\n            public void set(String key, Object value, Behaviors.Write write) {\n                values.put(key, new Behaviors.Entry(value, write.expiresAt));\n            }\n            public void delete(String key, Behaviors.Execution current) { values.remove(key); }\n        };\n        return new Behaviors.Configuration(java.util.Map.of(\"greetings\", cache), java.util.Map.of(\"greeting\", greeting), java.util.Map.of(), (call, current) -> true);\n    }\n}\n",
        ),
        (
            "client/tests/GreetingServiceTest.java",
            "@Test(GreetingTestSetup.class, {\"partition\":\"java-http-tests\"})\nclass GreetingServiceTest {\n    @Fake\n    GreetingClient greetingClient = null;\n    void testEmptyFakeUsesTheTestAdapter() {\n        GreetingService service = construct(GreetingService.class);\n        service.ask();\n        Assert.equals(\"Test adapter wins\", service.message);\n    }\n    void testListAnswersAreTyped() {\n        GreetingService service = construct(GreetingService.class);\n        Assert.equals(2, service.count());\n    }\n    void testAnAssignedFakeIsUsedAsIs() {\n        greetingClient = new GreetingClient();\n        GreetingService service = construct(GreetingService.class);\n        String code = \"\";\n        try { service.count(); } catch (Behaviors.Failure error) { code = error.code; }\n        Assert.equals(\"http-adapter\", code);\n    }\n}\n",
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
fn java_tests_construct_services_with_injected_fake_clients_and_typed_answers() {
    if std::process::Command::new("javac")
        .arg("-version")
        .output()
        .is_err()
    {
        assert!(std::env::var_os("CI").is_none(), "CI must provide javac");
        eprintln!("skipping: javac is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("test project");
    JavaHttpConstructProject::write(root.path());
    let report = super::run(
        root.path(),
        Path::new("client/tests/GreetingServiceTest.java"),
    )
    .expect("native Java test unit");
    assert_eq!(report.cases.len(), 3, "{report:?}");
    assert!(report.cases.iter().all(|case| case.passed), "{report:?}");
}
