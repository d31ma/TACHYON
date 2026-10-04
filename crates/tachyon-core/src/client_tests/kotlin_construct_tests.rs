#![allow(clippy::expect_used)]

use std::path::Path;

struct KotlinConstructProject;

impl KotlinConstructProject {
    const FILES: [(&'static str, &'static str); 6] = [
        (
            "client/models/Greeting.kt",
            "data class Greeting(val message: String)\n",
        ),
        (
            "client/remotes/GreetingClient.kt",
            "import models.Greeting\n\n@HTTPClient(\"/api\")\ninterface GreetingClient {\n    @GetRequest(\"/greeting\")\n    suspend fun getGreeting(): Greeting\n}\n",
        ),
        (
            "client/setups/GreetingSetup.kt",
            "@Setup\nclass GreetingSetup {\n    companion object {\n        fun configure(execution: Behaviors.Execution): Behaviors.Configuration = Behaviors.Configuration()\n    }\n}\n",
        ),
        (
            "client/services/CounterService.kt",
            "@Service\nclass CounterService {\n    @Needs\n    private val greetingClient: GreetingClient\n    var count: Int = 0\n    var message: String = \"\"\n\n    fun increment() {\n        count += 1\n    }\n\n    suspend fun askServer() {\n        message = greetingClient.getGreeting().message\n    }\n}\n",
        ),
        (
            "client/tests/HelloTestSetup.kt",
            "@TestSetup\nclass HelloTestSetup {\n    companion object {\n        fun configure(execution: Behaviors.Execution): Behaviors.Configuration =\n            Behaviors.Configuration(http = mapOf(\"greeting\" to Behaviors.HttpAdapter { _, _ -> mapOf(\"message\" to \"Hi\") }))\n    }\n}\n",
        ),
        (
            "client/tests/CounterServiceTest.kt",
            "@Test(HelloTestSetup::class, {\"partition\":\"tests\"})\nclass CounterServiceTest {\n    @Fake\n    var greetingClient: GreetingClient? = null\n\n    fun testIncrementAddsOne() {\n        val counter = construct(CounterService::class)\n        counter.increment()\n        Assert.equals(1, counter.count)\n    }\n\n    suspend fun testAskServerSavesTheTypedMessage() {\n        val counter = construct(CounterService::class)\n        counter.askServer()\n        Assert.equals(\"Hi\", counter.message)\n    }\n\n    suspend fun testAnAssignedFakeIsUsedAsIs() {\n        greetingClient = GreetingClient()\n        val counter = construct(CounterService::class)\n        val code = try { counter.askServer(); \"\" } catch (error: Behaviors.Failure) { error.code }\n        Assert.equals(\"HTTP_MISSING\", code)\n    }\n}\n",
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
fn kotlin_tests_construct_services_with_injected_fake_clients_and_typed_answers() {
    if std::process::Command::new("kotlinc")
        .arg("-version")
        .output()
        .is_err()
    {
        assert!(std::env::var_os("CI").is_none(), "CI must provide kotlinc");
        eprintln!("skipping: kotlinc is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("test project");
    KotlinConstructProject::write(root.path());
    let report = super::run(root.path(), Path::new("client/tests/CounterServiceTest.kt"))
        .expect("native Kotlin test unit");
    assert_eq!(report.cases.len(), 3, "{report:?}");
    assert!(report.cases.iter().all(|case| case.passed), "{report:?}");
}
