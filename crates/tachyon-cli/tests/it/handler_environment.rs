#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

struct HandlerEnvironment;

impl HandlerEnvironment {
    const REGION: (&'static str, &'static str) = ("HD_REGION", "hd-region-canary-7311");
    const TOKEN: (&'static str, &'static str) = ("APP_TOKEN", "app-token-canary-5521");
    const OTHER: (&'static str, &'static str) = ("OTHER_SECRET", "other-secret-canary-9043");
    const ALLOWLIST: &'static str = r#"["HD_*", "APP_TOKEN"]"#;
    const JAVASCRIPT: &'static str = r"
export class Handler {
  static GET() {
    return {
      region: process.env.HD_REGION ?? null,
      token: process.env.APP_TOKEN ?? null,
      other: process.env.OTHER_SECRET ?? null,
    }
  }
}
";
    const PYTHON: &'static str = r#"
import os


class Handler:
    @staticmethod
    def GET(request):
        return {
            "region": os.environ.get("HD_REGION"),
            "token": os.environ.get("APP_TOKEN"),
            "other": os.environ.get("OTHER_SECRET"),
        }
"#;
    const GO: &'static str = r#"package main

import "os"

type Handler struct{}

func (Handler) GET() any {
	answer := map[string]any{}
	for key, name := range map[string]string{"region": "HD_REGION", "token": "APP_TOKEN", "other": "OTHER_SECRET"} {
		if value, found := os.LookupEnv(name); found {
			answer[key] = value
		} else {
			answer[key] = nil
		}
	}
	return answer
}
"#;

    fn write(path: &Path, contents: &str) {
        fs::create_dir_all(path.parent().expect("fixture parent")).expect("fixture directory");
        fs::write(path, contents).expect("fixture source");
    }

    fn project(allowlist: Option<&str>) -> tempfile::TempDir {
        let project = tempfile::tempdir().expect("project");
        if let Some(allowlist) = allowlist {
            Self::write(
                &project.path().join("server/application.json"),
                &format!(r#"{{"server":{{"environment":{allowlist}}}}}"#),
            );
        }
        Self::write(
            &project.path().join("server/workers/environment.js"),
            Self::JAVASCRIPT,
        );
        Self::write(
            &project.path().join("server/workers/environment.py"),
            Self::PYTHON,
        );
        Self::write(
            &project.path().join("server/workers/environment.go"),
            Self::GO,
        );
        project
    }

    fn invoke(project: &Path, source: &str) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_ty"));
        command
            .args(["handler", "invoke", source])
            .arg("--project")
            .arg(project)
            .args(["--route", "/environment", "--method", "GET"]);
        let extension = Path::new(source).extension();
        if extension.is_some_and(|extension| extension.eq_ignore_ascii_case("py")) {
            command.args([
                "--python-runtime",
                if cfg!(windows) { "python" } else { "python3" },
            ]);
        }
        for (name, value) in [Self::REGION, Self::TOKEN, Self::OTHER] {
            command.env(name, value);
        }
        command.output().expect("the ty process should start")
    }

    fn body(output: &Output) -> serde_json::Value {
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let response: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("protocol response JSON");
        serde_json::from_str(response["body"]["data"].as_str().expect("body data"))
            .expect("handler JSON body")
    }

    fn assert_no_value_in(text: &str) {
        for (name, value) in [Self::REGION, Self::TOKEN, Self::OTHER] {
            assert!(!text.contains(value), "the value of {name} leaked: {text}");
        }
    }

    fn go_usable() -> bool {
        let found = Command::new("go")
            .arg("version")
            .output()
            .is_ok_and(|output| output.status.success());
        let required = cfg!(unix) && std::env::var_os("CI").is_some();
        assert!(
            found || !required,
            "go is required for the Go handler tests in CI"
        );
        found
    }

    fn sources() -> Vec<&'static str> {
        let mut sources = vec![
            "server/workers/environment.js",
            "server/workers/environment.py",
        ];
        if Self::go_usable() {
            sources.push("server/workers/environment.go");
        }
        sources
    }
}

#[test]
fn configured_variables_reach_javascript_python_and_go_handlers_and_others_do_not() {
    let project = HandlerEnvironment::project(Some(HandlerEnvironment::ALLOWLIST));
    for source in HandlerEnvironment::sources() {
        let output = HandlerEnvironment::invoke(project.path(), source);
        let body = HandlerEnvironment::body(&output);

        assert_eq!(body["region"], HandlerEnvironment::REGION.1, "{source}");
        assert_eq!(body["token"], HandlerEnvironment::TOKEN.1, "{source}");
        assert_eq!(body["other"], serde_json::Value::Null, "{source}");
        HandlerEnvironment::assert_no_value_in(&String::from_utf8_lossy(&output.stderr));
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(!stdout.contains(HandlerEnvironment::OTHER.1), "{stdout}");
    }
}

#[test]
fn without_configuration_handlers_get_no_application_variables() {
    let project = HandlerEnvironment::project(None);
    let output = HandlerEnvironment::invoke(project.path(), "server/workers/environment.js");
    let body = HandlerEnvironment::body(&output);

    assert_eq!(body["region"], serde_json::Value::Null);
    assert_eq!(body["token"], serde_json::Value::Null);
    assert_eq!(body["other"], serde_json::Value::Null);
}

#[test]
fn refused_patterns_stop_the_invocation_and_name_the_entry_but_no_value() {
    for (allowlist, entry, reason) in [
        (r#"["*"]"#, "\"*\"", "every variable"),
        (r#"["HD_*", "TAC_*"]"#, "\"TAC_*\"", "ty's own variables"),
        (
            r#"["HTTPS_PROXY"]"#,
            "\"HTTPS_PROXY\"",
            "ty's own variables",
        ),
    ] {
        let project = HandlerEnvironment::project(Some(allowlist));
        let output = HandlerEnvironment::invoke(project.path(), "server/workers/environment.js");
        let stderr = String::from_utf8_lossy(&output.stderr);

        assert!(!output.status.success(), "{allowlist}");
        assert!(stderr.contains("TY2010"), "{stderr}");
        assert!(stderr.contains(entry), "{stderr}");
        assert!(stderr.contains(reason), "{stderr}");
        HandlerEnvironment::assert_no_value_in(&stderr);
        HandlerEnvironment::assert_no_value_in(&String::from_utf8_lossy(&output.stdout));
    }
}

#[test]
fn test_units_get_the_configured_variables_and_no_others() {
    let project = HandlerEnvironment::project(Some(HandlerEnvironment::ALLOWLIST));
    HandlerEnvironment::write(
        &project.path().join("server/tests/EnvironmentTestSetup.js"),
        "@TestSetup()\nclass EnvironmentTestSetup {\n  static configure() { return {}; }\n}\n",
    );
    HandlerEnvironment::write(
        &project.path().join("server/tests/EnvironmentTest.js"),
        &format!(
            "@Test(EnvironmentTestSetup)\nexport default class EnvironmentTest {{\n  async testConfiguredVariables() {{\n    Assert.equals('{}', process.env.HD_REGION ?? null)\n    Assert.equals(null, process.env.OTHER_SECRET ?? null)\n  }}\n}}\n",
            HandlerEnvironment::REGION.1
        ),
    );
    let mut command = Command::new(env!("CARGO_BIN_EXE_ty"));
    command.arg("test").arg(project.path());
    for (name, value) in [HandlerEnvironment::REGION, HandlerEnvironment::OTHER] {
        command.env(name, value);
    }
    let output = command.output().expect("the ty process should start");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success(), "{stdout}\n{stderr}");
    assert!(stdout.contains("testConfiguredVariables"), "{stdout}");
}
