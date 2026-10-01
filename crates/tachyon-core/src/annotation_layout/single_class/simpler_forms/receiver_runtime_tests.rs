#![allow(clippy::expect_used)]

use std::path::Path;

struct GroovyMemberCheck;

impl GroovyMemberCheck {
    const SERVICE: &str = "@Service\nclass CounterService {\n    int count = 0\n    void increment() { count += 1 }\n}\n";
    const SETUP: &str = "@TestSetup\nclass CounterTestSetup {\n    static Behaviors.Configuration configure(Behaviors.Execution execution) {\n        return new Behaviors.Configuration([:], [:], [:], null, null)\n    }\n}\n";
    const UNIT: &str = "@Test(CounterTestSetup.class, {\"partition\":\"tests\"})\nclass CounterServiceTest {\n    void testBareMembersUpdateCount() {\n        CounterService counter = new CounterService()\n        counter.increment()\n        counter.increment()\n        Assert.equals(2, counter.count)\n    }\n}\n";

    async fn run() {
        let root = tempfile::tempdir().expect("Groovy project");
        Self::write(
            root.path(),
            "server/services/CounterService.groovy",
            Self::SERVICE,
        );
        Self::write(
            root.path(),
            "server/tests/CounterTestSetup.groovy",
            Self::SETUP,
        );
        Self::write(
            root.path(),
            "server/tests/CounterServiceTest.groovy",
            Self::UNIT,
        );
        let source =
            crate::HandlerSource::discover(root.path(), "server/tests/CounterServiceTest.groovy")
                .expect("Groovy test discovery");
        let supervisor = crate::HandlerSupervisor::new(crate::HandlerSupervisorOptions::default())
            .expect("supervised Groovy runtime");
        let response = supervisor
            .invoke(
                &source,
                &tachyon_contracts::HandlerRequest::route(
                    "bare_members",
                    "/",
                    tachyon_contracts::HttpMethod::Post,
                ),
                &crate::HandlerCancellation::default(),
            )
            .await
            .expect("Groovy runtime invocation");
        assert!(response.error.is_none(), "{:?}", response.error);
        let body = response.body.expect("Groovy result").data;
        let report: serde_json::Value = serde_json::from_str(&body).expect("Groovy JSON result");
        let cases = report["cases"].as_array().expect("Groovy cases");
        assert_eq!(cases.len(), 1, "{body}");
        assert_eq!(cases[0]["passed"], true, "{body}");
        assert!(
            cases[0]["called"]
                .as_array()
                .expect("actual recorded calls")
                .contains(&serde_json::Value::String(String::from(
                    "CounterService.increment"
                ))),
            "{body}"
        );
    }

    fn write(root: &Path, relative: &str, source: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("fixture parent")).expect("fixture directory");
        std::fs::write(path, source).expect("Groovy fixture");
    }
}

#[tokio::test]
#[ignore = "requires Groovy/JDK SDKs; explicit runtime qualification"]
async fn groovy_bare_service_calls_are_recorded_by_runtime() {
    GroovyMemberCheck::run().await;
}
