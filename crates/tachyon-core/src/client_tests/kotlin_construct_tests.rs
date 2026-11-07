#![allow(clippy::expect_used)]

use std::path::Path;

struct KotlinConstructProject;

impl KotlinConstructProject {
    const FILES: [(&'static str, &'static str); 6] = [
        (
            "client/models/Greeting.kt",
            include_str!(
                "../../../../website/content/docs/first-app/call-the-server/client/models/Greeting.kt"
            ),
        ),
        (
            "client/remotes/GreetingClient.kt",
            include_str!(
                "../../../../website/content/docs/first-app/call-the-server/client/remotes/GreetingClient.kt"
            ),
        ),
        (
            "client/setups/GreetingSetup.kt",
            include_str!(
                "../../../../website/content/docs/first-app/call-the-server/client/setups/GreetingSetup.kt"
            ),
        ),
        (
            "client/services/CounterService.kt",
            include_str!(
                "../../../../website/content/docs/first-app/call-the-server/client/services/CounterService.kt"
            ),
        ),
        (
            "client/tests/HelloTestSetup.kt",
            include_str!(
                "../../../../website/content/docs/first-app/add-tests/client/tests/HelloTestSetup.kt"
            ),
        ),
        (
            "client/tests/AssignedFakeTest.kt",
            "@Test(HelloTestSetup::class, {\"partition\":\"tests\"})\nclass AssignedFakeTest {\n    @Fake\n    var greetingClient: GreetingClient? = null\n\n    suspend fun testAnAssignedFakeIsUsedAsIs() {\n        greetingClient = GreetingClient()\n        val counter = construct(CounterService::class)\n        val code = try { counter.askServer(); \"\" } catch (error: Behaviors.Failure) { error.code }\n        Assert.equals(\"HTTP_MISSING\", code)\n    }\n}\n",
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
    if crate::kotlin_home::KotlinHome::command("kotlinc")
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
    let report = super::run(root.path(), Path::new("client/tests/AssignedFakeTest.kt"))
        .expect("native Kotlin test unit");
    assert_eq!(report.cases.len(), 1, "{report:?}");
    assert!(report.cases[0].passed, "{report:?}");
}
