#![allow(clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::process::Command;

struct FyloProject {
    workspace: tempfile::TempDir,
}

impl FyloProject {
    const HANDLER_TIMEOUT_MS: &'static str = "60000";
    const WINDOWS_SLOW_WRITE_TIMEOUT_MS: &'static str = "240000";
    const GO_FILES: [(&'static str, &'static str); 5] = [
        (
            "server/models/Event.go",
            "type Event struct {\n\tID    string `json:\"id,omitempty\"`\n\tKind  string `json:\"kind\"`\n\tCount int    `json:\"count\"`\n}\n",
        ),
        (
            "server/models/Job.go",
            "type Job struct {\n\tApp string `json:\"app\"`\n}\n",
        ),
        (
            "server/models/Report.go",
            "type Report struct {\n\tFound   Event `json:\"found\"`\n\tPatched Event `json:\"patched\"`\n\tScrolled int `json:\"scrolled\"`\n\tPages int `json:\"pages\"`\n\tJob Job `json:\"job\"`\n\tGone bool `json:\"gone\"`\n\tDropped bool `json:\"dropped\"`\n\tExists bool `json:\"exists\"`\n}\n",
        ),
        (
            "server/services/EventService.go",
            r#"// @Service
type EventService struct {
	// @Needs
	dataSource DataSource
}

func (service *EventService) Seed(count int) (string, error) {
	if err := service.dataSource.CreateCollection("hd-events"); err != nil {
		return "", err
	}
	first := ""
	for index := 0; index < count; index++ {
		id, err := service.dataSource.Insert("hd-events", Event{Kind: "deploy", Count: index})
		if err != nil {
			return "", err
		}
		if first == "" {
			first = id
		}
	}
	return first, nil
}

func (service *EventService) ReadAndPatch(id string, report *Report) error {
	if _, err := service.dataSource.FindById("hd-events", id, &report.Found); err != nil {
		return err
	}
	_, err := service.dataSource.Patch("hd-events", id, map[string]any{"count": 99}, &report.Patched)
	return err
}

func (service *EventService) ScrollAll(report *Report) error {
	cursor := ""
	for {
		window, err := service.dataSource.Scroll("hd-events", map[string]any{"kind": "deploy"}, cursor, 5)
		if err != nil {
			return err
		}
		events := []Event{}
		if err := window.Decode(&events); err != nil {
			return err
		}
		report.Scrolled += len(events)
		report.Pages++
		cursor = window.NextCursor
		if cursor == "" {
			return nil
		}
	}
}

func (service *EventService) RunQueue(report *Report) error {
	if _, err := service.dataSource.Publish("app.render", Job{App: "web"}, "render:web"); err != nil {
		return err
	}
	deliveries, err := service.dataSource.Claim("app.render", "renderers", 5, 30000)
	if err != nil || len(deliveries) != 1 {
		return err
	}
	if err := deliveries[0].Decode(&report.Job); err != nil {
		return err
	}
	return service.dataSource.Ack("app.render", "renderers", deliveries[0])
}

func (service *EventService) CleanUp(id string, report *Report) error {
	if _, err := service.dataSource.DeleteById("hd-events", id); err != nil {
		return err
	}
	found, err := service.dataSource.FindById("hd-events", id, nil)
	if err != nil {
		return err
	}
	report.Gone = !found
	if report.Dropped, err = service.dataSource.DropCollection("hd-events"); err != nil {
		return err
	}
	report.Exists, err = service.dataSource.CollectionExists("hd-events")
	return err
}
"#,
        ),
        (
            "server/controllers/EventsController.go",
            r#"// @RequestRoute("/events")
// @Public
type EventsController struct {
	// @Needs
	events *EventService
}

// @GetRoute
func (controller *EventsController) GetEvents() (Report, error) {
	report := Report{}
	id, err := controller.events.Seed(12)
	if err != nil {
		return report, err
	}
	if err := controller.events.ReadAndPatch(id, &report); err != nil {
		return report, err
	}
	if err := controller.events.ScrollAll(&report); err != nil {
		return report, err
	}
	if err := controller.events.RunQueue(&report); err != nil {
		return report, err
	}
	return report, controller.events.CleanUp(id, &report)
}
"#,
        ),
    ];

    const PYTHON_FILES: [(&'static str, &'static str); 2] = [
        (
            "server/services/EventService.py",
            r#"from typing import Annotated

@service
class EventService:
    data_source: Annotated[DataSource, needs]

    def exercise(self) -> dict[str, object]:
        self.data_source.create_collection("hd-events")
        saved = [self.data_source.insert("hd-events", {"kind": "deploy", "count": index}) for index in range(12)]
        first = saved[0]["id"]
        found = self.data_source.find_by_id("hd-events", first)
        patched = self.data_source.patch("hd-events", first, {"count": 99})
        scrolled, pages, cursor = 0, 0, None
        while True:
            window = self.data_source.scroll("hd-events", {"kind": "deploy"}, cursor, 5)
            scrolled += len(window["content"])
            pages += 1
            cursor = window["nextCursor"]
            if cursor is None:
                break
        self.data_source.publish("app.render", {"app": "web"}, "render:web")
        delivery = self.data_source.claim("app.render", "renderers", 5, 30000)[0]
        self.data_source.ack("app.render", "renderers", delivery)
        self.data_source.delete_by_id("hd-events", first)
        gone = self.data_source.find_by_id("hd-events", first) is None
        dropped = self.data_source.drop_collection("hd-events")
        exists = self.data_source.collection_exists("hd-events")
        return {"found": found, "patched": patched, "scrolled": scrolled, "pages": pages,
                "job": delivery["payload"], "gone": gone, "dropped": dropped, "exists": exists}
"#,
        ),
        (
            "server/controllers/EventsController.py",
            r#"from typing import Annotated

@request_route("/events")
@public
class EventsController:
    events: Annotated[EventService, needs]

    @get_route
    def get_events(self) -> dict[str, object]:
        return self.events.exercise()
"#,
        ),
    ];

    fn new(files: &[(&str, &str)]) -> Self {
        let workspace = tempfile::tempdir().expect("workspace");
        let root = workspace.path().join("fylo-root");
        std::fs::create_dir_all(&root).expect("FYLO root");
        let root = std::fs::canonicalize(&root).expect("canonical FYLO root");
        let url = root.display().to_string().replace('\\', "/");
        let url = url.strip_prefix("//?/").unwrap_or(&url);
        let settings = serde_json::json!({
            "datasource": {"driver": "fylo", "url": format!("fylo:///{}", url.trim_start_matches('/'))}
        });
        let project = workspace.path().join("project");
        Self::write(
            &project.join("server/application.json"),
            &settings.to_string(),
        );
        for (path, source) in files {
            Self::write(&project.join(path), source);
        }
        Self { workspace }
    }

    fn write(path: &Path, contents: &str) {
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(path, contents).expect("fixture");
    }

    fn project(&self) -> PathBuf {
        self.workspace.path().join("project")
    }

    fn report(&self, controller: &str) -> serde_json::Value {
        let mut command = Command::new(env!("CARGO_BIN_EXE_ty"));
        command
            .args(["handler", "invoke", controller])
            .arg("--project")
            .arg(self.project())
            .args([
                "--route",
                "/events",
                "--method",
                "GET",
                "--timeout-ms",
                Self::timeout_ms(),
            ]);
        let extension = Path::new(controller).extension();
        if extension.is_some_and(|extension| extension.eq_ignore_ascii_case("py")) {
            command.args([
                "--python-runtime",
                if cfg!(windows) { "python" } else { "python3" },
            ]);
        }
        let output = command.output().expect("ty handler invoke");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let response: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("handler response");
        assert_eq!(response["status"], 200, "{response}");
        let body = response["body"]["data"].as_str().expect("JSON body");
        serde_json::from_str(body).expect("report JSON")
    }

    fn timeout_ms() -> &'static str {
        if cfg!(windows) {
            Self::WINDOWS_SLOW_WRITE_TIMEOUT_MS
        } else {
            Self::HANDLER_TIMEOUT_MS
        }
    }

    fn available(program: &str) -> bool {
        let found = Command::new(program)
            .arg(if program == "go" {
                "version"
            } else {
                "--version"
            })
            .output()
            .is_ok_and(|output| output.status.success());
        if !found {
            assert!(
                std::env::var("TAC_STORAGE_REQUIRE_TOOLCHAINS").is_err(),
                "{program} is required on this runner"
            );
        }
        found
    }

    fn assert_report(report: &serde_json::Value) {
        assert_eq!(report["found"]["kind"], "deploy", "{report}");
        assert_eq!(report["found"]["count"], 0, "{report}");
        assert_eq!(report["patched"]["count"], 99, "{report}");
        assert_eq!(report["patched"]["id"], report["found"]["id"], "{report}");
        assert_eq!(report["scrolled"], 12, "{report}");
        assert_eq!(report["pages"], 3, "{report}");
        assert_eq!(report["job"], serde_json::json!({"app": "web"}), "{report}");
        assert_eq!(report["gone"], true, "{report}");
        assert_eq!(report["dropped"], true, "{report}");
        assert_eq!(report["exists"], false, "{report}");
    }
}

#[test]
fn go_services_use_every_fylo_operation_with_typed_models() {
    if !FyloProject::available("fylo") || !FyloProject::available("go") {
        return;
    }
    let project = FyloProject::new(&FyloProject::GO_FILES);
    let report = project.report("server/controllers/EventsController.go");
    FyloProject::assert_report(&report);
}

#[test]
fn python_services_use_every_fylo_operation() {
    if !FyloProject::available("fylo") || !FyloProject::available("python3") {
        return;
    }
    let project = FyloProject::new(&FyloProject::PYTHON_FILES);
    let report = project.report("server/controllers/EventsController.py");
    FyloProject::assert_report(&report);
}
