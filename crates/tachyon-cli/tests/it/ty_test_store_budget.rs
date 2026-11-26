#![allow(clippy::expect_used)]

use crate::windows_egress::WindowsEgress;
use std::path::PathBuf;
use std::process::Command;

struct StoreBudgetProject {
    workspace: tempfile::TempDir,
}

impl StoreBudgetProject {
    const GO_SERVICE: &'static str = "// @Service
type EventsService struct{}

func (service *EventsService) Write(number int) error {
	_, err := DataSource{}.Insert(\"hd-events\", map[string]any{\"n\": number})
	return err
}
";

    const GO_SETUP: &'static str = "// @TestSetup
type EventsTestSetup struct{}

func (EventsTestSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	return NewBehaviorConfiguration(), nil
}
";

    const GO_UNIT: &'static str = "import \"time\"

// @Test(EventsTestSetup, {\"partition\":\"tests\"})
type EventsServiceTest struct {
	behaviors *BehaviorContext
}

func (test *EventsServiceTest) writesForOneSecond() bool {
	if (DataSource{}).CreateCollection(\"hd-events\") != nil {
		return false
	}
	started := time.Now()
	for number := 0; time.Since(started) < time.Second; number++ {
		if (&EventsService{}).Write(number) != nil {
			return false
		}
	}
	return true
}

func (test *EventsServiceTest) TestTheFirstSecondOfWrites() {
	Assert.IsTrue(test.writesForOneSecond(), \"every write saved\")
}

func (test *EventsServiceTest) TestTheSecondSecondOfWrites() {
	Assert.IsTrue(test.writesForOneSecond(), \"every write saved\")
}

func (test *EventsServiceTest) TestTheThirdSecondOfWrites() {
	Assert.IsTrue(test.writesForOneSecond(), \"every write saved\")
}

func (test *EventsServiceTest) TestTheFourthSecondOfWrites() {
	Assert.IsTrue(test.writesForOneSecond(), \"every write saved\")
}

func (test *EventsServiceTest) TestTheFifthSecondOfWrites() {
	Assert.IsTrue(test.writesForOneSecond(), \"every write saved\")
}
";

    fn new() -> Self {
        let workspace = tempfile::tempdir().expect("workspace");
        std::fs::create_dir_all(workspace.path().join("fylo-root")).expect("FYLO root");
        let project = Self { workspace };
        let datasource =
            serde_json::json!({"driver": "fylo", "url": project.root_url(), "timeout": 3000});
        let application = serde_json::json!({ "datasource": datasource });
        project.write("server/application.json", &application.to_string());
        project.write("server/services/EventsService.go", Self::GO_SERVICE);
        project.write("server/tests/EventsTestSetup.go", Self::GO_SETUP);
        project.write("server/tests/EventsServiceTest.go", Self::GO_UNIT);
        project
    }

    fn write(&self, relative: &str, contents: &str) {
        let path = self.project().join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(path, contents).expect("fixture");
    }

    fn project(&self) -> PathBuf {
        self.workspace.path().join("project")
    }

    fn root_url(&self) -> String {
        let root =
            std::fs::canonicalize(self.workspace.path().join("fylo-root")).expect("FYLO root");
        let url = root.display().to_string().replace('\\', "/");
        let url = url.strip_prefix("//?/").unwrap_or(&url).to_owned();
        format!("fylo:///{}", url.trim_start_matches('/'))
    }

    fn tools_present() -> bool {
        let fylo = Self::runs("fylo", "--version");
        let go = Self::runs("go", "version");
        assert!(
            (fylo && go) || std::env::var("TAC_STORAGE_REQUIRE_TOOLCHAINS").is_err(),
            "fylo and go are required on this runner"
        );
        fylo && go
    }

    fn runs(program: &str, argument: &str) -> bool {
        Command::new(program)
            .arg(argument)
            .output()
            .is_ok_and(|output| output.status.success())
    }

    fn test(&self) -> (bool, String) {
        let mut command = Command::new(env!("CARGO_BIN_EXE_ty"));
        command
            .arg("test")
            .arg(self.project())
            .env_remove("YON_PROFILE");
        WindowsEgress::proxied_where_unconfinable(&mut command);
        let output = command.output().expect("ty test");
        let report = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        (output.status.success(), report)
    }
}

#[test]
fn ty_test_gives_each_store_call_of_a_test_unit_its_own_budget() {
    if !StoreBudgetProject::tools_present() {
        return;
    }
    let (passed, report) = StoreBudgetProject::new().test();
    assert!(passed, "{report}");
    assert!(!report.contains("store-time budget is spent"), "{report}");
    assert_eq!(
        report.matches("ok  EventsServiceTest.").count(),
        5,
        "{report}"
    );
}
