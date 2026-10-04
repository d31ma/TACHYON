#![allow(clippy::expect_used)]

use std::path::Path;

struct GoConstructProject;

impl GoConstructProject {
    const FILES: [(&'static str, &'static str); 6] = [
        (
            "client/models/Greeting.go",
            "type Greeting struct {\n\tMessage string `json:\"message\"`\n}\n",
        ),
        (
            "client/remotes/GreetingClient.go",
            "// @HTTPClient(\"/api\")\ntype GreetingClient interface {\n\t// @GetRequest(\"/greeting\")\n\tGetGreeting() (Greeting, error)\n}\n",
        ),
        (
            "client/setups/GreetingSetup.go",
            "// @Setup\ntype GreetingSetup struct{}\n\nfunc (GreetingSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {\n\treturn NewBehaviorConfiguration(), nil\n}\n",
        ),
        (
            "client/services/CounterService.go",
            "// @Service\ntype CounterService struct {\n\t// @Needs\n\tgreetingClient *GreetingClient\n\tCount          int\n\tMessage        string\n}\n\nfunc (counter *CounterService) Increment() {\n\tcounter.Count += 1\n}\n\nfunc (counter *CounterService) AskServer() error {\n\tgreeting, err := counter.greetingClient.GetGreeting()\n\tcounter.Message = greeting.Message\n\treturn err\n}\n",
        ),
        (
            "client/tests/HelloTestSetup.go",
            "// @TestSetup\ntype HelloTestSetup struct{}\n\nfunc (HelloTestSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {\n\tconfig := NewBehaviorConfiguration()\n\tconfig.Http[\"greeting\"] = HelloTestSetup{}\n\treturn config, nil\n}\n\nfunc (HelloTestSetup) Exchange(request *BehaviorHttpRequest, execution *BehaviorExecution) (BehaviorValue, error) {\n\treturn map[string]any{\"message\": \"Hi\"}, nil\n}\n",
        ),
        (
            "client/tests/CounterServiceTest.go",
            "// @Test(HelloTestSetup, {\"partition\":\"tests\"})\ntype CounterServiceTest struct {\n\tbehaviors *BehaviorContext\n\t// @Fake\n\tgreetingClient *GreetingClient\n}\n\nfunc (test *CounterServiceTest) TestIncrementAddsOne() {\n\tcounter := &CounterService{}\n\ttest.construct(counter)\n\tcounter.Increment()\n\tAssert.Equals(1, counter.Count)\n}\n\nfunc (test *CounterServiceTest) TestAskServerSavesTheTypedMessage() {\n\tcounter := &CounterService{}\n\ttest.construct(counter)\n\tAssert.Equals(nil, counter.AskServer())\n\tAssert.Equals(\"Hi\", counter.Message)\n}\n\nfunc (test *CounterServiceTest) TestAnAssignedFakeIsUsedAsIs() {\n\tclient, _ := NewGreetingClient()\n\ttest.greetingClient = client\n\tcounter := &CounterService{}\n\ttest.construct(counter)\n\tAssert.Equals(\"HTTP_MISSING\", BehaviorErrorCode(counter.AskServer()))\n}\n",
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
    let report = super::run(root.path(), Path::new("client/tests/CounterServiceTest.go"))
        .expect("native Go test unit");
    assert_eq!(report.cases.len(), 3, "{report:?}");
    assert!(report.cases.iter().all(|case| case.passed), "{report:?}");
}
