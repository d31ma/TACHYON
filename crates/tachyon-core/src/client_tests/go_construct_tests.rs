#![allow(clippy::expect_used)]

use std::path::Path;

struct GoConstructProject;

impl GoConstructProject {
    const FILES: [(&'static str, &'static str); 6] = [
        (
            "client/models/Greeting.go",
            include_str!(
                "../../../../website/content/docs/first-app/call-the-server/client/models/Greeting.go"
            ),
        ),
        (
            "client/remotes/GreetingClient.go",
            include_str!(
                "../../../../website/content/docs/first-app/call-the-server/client/remotes/GreetingClient.go"
            ),
        ),
        (
            "client/setups/GreetingSetup.go",
            include_str!(
                "../../../../website/content/docs/first-app/call-the-server/client/setups/GreetingSetup.go"
            ),
        ),
        (
            "client/services/CounterService.go",
            include_str!(
                "../../../../website/content/docs/first-app/call-the-server/client/services/CounterService.go"
            ),
        ),
        (
            "client/tests/HelloTestSetup.go",
            include_str!(
                "../../../../website/content/docs/first-app/add-tests/client/tests/HelloTestSetup.go"
            ),
        ),
        (
            "client/tests/AssignedFakeTest.go",
            "// @Test(HelloTestSetup, {\"partition\":\"tests\"})\ntype AssignedFakeTest struct {\n\tbehaviors *BehaviorContext\n\t// @Fake\n\tgreetingClient *GreetingClient\n}\n\nfunc (test *AssignedFakeTest) TestAnAssignedFakeIsUsedAsIs() {\n\tclient, _ := NewGreetingClient()\n\ttest.greetingClient = client\n\tcounter := &CounterService{}\n\ttest.construct(counter)\n\tAssert.Equals(\"HTTP_MISSING\", BehaviorErrorCode(counter.AskServer()))\n}\n",
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
fn go_tests_construct_services_with_injected_fake_clients_and_typed_answers() {
    if std::process::Command::new("go")
        .arg("version")
        .output()
        .is_err()
    {
        assert!(std::env::var_os("CI").is_none(), "CI must provide go");
        eprintln!("skipping: go is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("test project");
    GoConstructProject::write(root.path());
    let report = super::run(root.path(), Path::new("client/tests/AssignedFakeTest.go"))
        .expect("native Go test unit");
    assert_eq!(report.cases.len(), 1, "{report:?}");
    assert!(report.cases[0].passed, "{report:?}");
}
