#![allow(clippy::expect_used)]

use crate::windows_egress::WindowsEgress;
use std::fs;
use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::net::TcpStream;
use std::path::Path;
use std::process::{ChildStdout, Command, Output, Stdio};
use std::sync::mpsc;
use std::time::Duration;

struct LogCase {
    extension: &'static str,
    program: &'static str,
    flag: &'static str,
    controller: &'static str,
}

impl LogCase {
    const INSTALLED_IN_CI: [&'static str; 12] = [
        "php-cgi", "groovyc", "kotlinc", "dart", "lua", "go", "ruby", "rustc", "node", "clang++",
        "dotnet", "javac",
    ];

    const JAVASCRIPT: Self = Self {
        extension: "js",
        program: "node",
        flag: "--version",
        controller: "@RequestRoute('/logs')\n@Public\nexport class LogsController {\n  /** @returns {string[]} */\n  @GetRoute\n  getLogs() {\n    YonLog.debug('hidden detail')\n    YonLog.info('slow upstream', { upstream_ms: 812 })\n    return ['ok']\n  }\n}\n",
    };

    const TYPESCRIPT: Self = Self {
        extension: "ts",
        program: "node",
        flag: "--version",
        controller: "@RequestRoute(\"/logs\")\n@Public\nclass LogsController {\n  @GetRoute\n  getLogs(): string[] {\n    YonLog.debug(\"hidden detail\")\n    YonLog.info(\"slow upstream\", { upstream_ms: 812 })\n    return [\"ok\"]\n  }\n}\n",
    };

    const PYTHON: Self = Self {
        extension: "py",
        program: if cfg!(windows) { "python" } else { "python3" },
        flag: "--version",
        controller: "@request_route(\"/logs\")\n@public\nclass LogsController:\n    @get_route\n    def get_logs(self) -> list[str]:\n        YonLog.debug(\"hidden detail\")\n        YonLog.info(\"slow upstream\", {\"upstream_ms\": 812})\n        return [\"ok\"]\n",
    };

    const RUBY: Self = Self {
        extension: "rb",
        program: "ruby",
        flag: "--version",
        controller: "#[RequestRoute(\"/logs\")]\n#[Public]\nclass LogsController\n  # @return [Array<String>]\n  #[GetRoute]\n  def get_logs\n    YonLog.debug(\"hidden detail\")\n    YonLog.info(\"slow upstream\", { \"upstream_ms\" => 812 })\n    [\"ok\"]\n  end\nend\n",
    };

    const LUA: Self = Self {
        extension: "lua",
        program: "lua",
        flag: "-v",
        controller: "-- @RequestRoute(\"/logs\")\n-- @Public\n---@class LogsController\nLogsController = {}\n\n---@return string[]\n-- @GetRoute\nfunction LogsController:get_logs()\n  YonLog.debug(\"hidden detail\")\n  YonLog.info(\"slow upstream\", { upstream_ms = 812 })\n  return { \"ok\" }\nend\n",
    };

    const PHP: Self = Self {
        extension: "php",
        program: "php-cgi",
        flag: "--version",
        controller: "<?php\n#[RequestRoute(\"/logs\")]\n#[Public]\nclass LogsController {\n    #[GetRoute]\n    public function getLogs(): array {\n        YonLog::debug('hidden detail');\n        YonLog::info('slow upstream', ['upstream_ms' => 812]);\n        return ['ok'];\n    }\n}\n",
    };

    const JAVA: Self = Self {
        extension: "java",
        program: "javac",
        flag: "--version",
        controller: "@RequestRoute(\"/logs\")\n@Public\nclass LogsController {\n    @GetRoute\n    java.util.List<String> getLogs() {\n        YonLog.debug(\"hidden detail\");\n        YonLog.info(\"slow upstream\", java.util.Map.of(\"upstream_ms\", 812));\n        return java.util.List.of(\"ok\");\n    }\n}\n",
    };

    const KOTLIN: Self = Self {
        extension: "kt",
        program: "kotlinc",
        flag: "-version",
        controller: "@RequestRoute(\"/logs\")\n@Public\nclass LogsController {\n    @GetRoute\n    fun getLogs(): List<String> {\n        YonLog.debug(\"hidden detail\")\n        YonLog.info(\"slow upstream\", mapOf(\"upstream_ms\" to 812))\n        return listOf(\"ok\")\n    }\n}\n",
    };

    const GROOVY: Self = Self {
        extension: "groovy",
        program: "groovyc",
        flag: "--version",
        controller: "@RequestRoute('/logs')\n@Public\nclass LogsController {\n    @GetRoute\n    List<String> getLogs() {\n        YonLog.debug('hidden detail')\n        YonLog.info('slow upstream', [upstream_ms: 812])\n        return ['ok']\n    }\n}\n",
    };

    const CSHARP: Self = Self {
        extension: "cs",
        program: "dotnet",
        flag: "--version",
        controller: "using System.Collections.Generic;\n\n[RequestRoute(\"/logs\")]\n[Public]\nclass LogsController {\n    [GetRoute]\n    public List<string> GetLogs() {\n        YonLog.Debug(\"hidden detail\");\n        YonLog.Info(\"slow upstream\", new Dictionary<string, object?> { [\"upstream_ms\"] = 812 });\n        return new List<string> { \"ok\" };\n    }\n}\n",
    };

    const FSHARP: Self = Self {
        extension: "fs",
        program: "dotnet",
        flag: "--version",
        controller: "[<RequestRoute(\"/logs\")>]\n[<Public>]\ntype LogsController() =\n    [<GetRoute>]\n    member this.GetLogs() : string list =\n        YonLog.Debug(\"hidden detail\")\n        YonLog.Info(\"slow upstream\", dict [ \"upstream_ms\", box 812 ])\n        [ \"ok\" ]\n",
    };

    const RUST: Self = Self {
        extension: "rs",
        program: "rustc",
        flag: "--version",
        controller: "#[RequestRoute(\"/logs\")]\n#[Public]\nstruct LogsController {}\nimpl LogsController {\n    fn new() -> Self { Self {} }\n\n    #[GetRoute]\n    fn get_logs(&self) -> Vec<String> {\n        YonLog::debug(\"hidden detail\", ());\n        YonLog::info(\"slow upstream\", Json::map([(\"upstream_ms\", 812)]));\n        vec![String::from(\"ok\")]\n    }\n}\n",
    };

    const DART: Self = Self {
        extension: "dart",
        program: "dart",
        flag: "--version",
        controller: "@RequestRoute(\"/logs\")\n@Public\nclass LogsController {\n  @GetRoute\n  List<String> getLogs() {\n    YonLog.debug('hidden detail');\n    YonLog.info('slow upstream', {'upstream_ms': 812});\n    return ['ok'];\n  }\n}\n",
    };

    const GO: Self = Self {
        extension: "go",
        program: "go",
        flag: "version",
        controller: "// @RequestRoute(\"/logs\")\n// @Public\ntype LogsController struct {\n}\n\n// @GetRoute\nfunc (controller *LogsController) GetLogs() []string {\n\tYonLog.Debug(\"hidden detail\")\n\tYonLog.Info(\"slow upstream\", map[string]any{\"upstream_ms\": 812})\n\treturn []string{\"ok\"}\n}\n",
    };

    const CPP: Self = Self {
        extension: "hpp",
        program: "clang++",
        flag: "--version",
        controller: "#pragma once\n#include <string>\n#include <vector>\n\nclass [[RequestRoute(\"/logs\")]] [[Public]] LogsController {\npublic:\n    [[GetRoute]]\n    std::vector<std::string> getLogs() {\n        tachyon::yon::YonLog::debug(\"hidden detail\");\n        tachyon::yon::YonLog::info(\"slow upstream\", {{\"upstream_ms\", 812}});\n        return {std::string(\"ok\")};\n    }\n};\n",
    };

    fn entry(&self) -> String {
        format!("server/controllers/LogsController.{}", self.extension)
    }

    fn available(&self) -> bool {
        let found = Command::new(self.program)
            .arg(self.flag)
            .output()
            .is_ok_and(|output| output.status.success());
        let required = [
            std::env::var_os("CI").is_some(),
            Self::INSTALLED_IN_CI.contains(&self.program),
        ] == [true, true];
        assert!(
            [required, found] != [true, false],
            "{} is installed in CI, so this test must run there instead of skipping",
            self.program
        );
        found
    }

    fn project(&self) -> tempfile::TempDir {
        let project = tempfile::tempdir().expect("project");
        HandlerLogs::write(&project.path().join(self.entry()), self.controller);
        project
    }

    fn invoke(&self, project: &Path, level: Option<&str>) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_ty"));
        command
            .args(["handler", "invoke", &self.entry()])
            .arg("--project")
            .arg(project)
            .args(["--route", "/logs", "--method", "get"])
            .env_remove("TAC_HANDLER_STDERR")
            .env_remove("TAC_HANDLER_LOG_LEVEL");
        if self.extension == "py" {
            command.args(["--python-runtime", self.program]);
        }
        if let Some(level) = level {
            command.env("TAC_HANDLER_LOG_LEVEL", level);
        }
        command.output().expect("the ty process should start")
    }

    fn assert_logs(&self) {
        if !self.available() {
            eprintln!(
                "skipping {}: {} is unavailable",
                self.extension, self.program
            );
            return;
        }
        let project = self.project();
        let output = self.invoke(project.path(), None);
        let stderr = String::from_utf8_lossy(&output.stderr);
        HandlerLogs::assert_answer(&output);
        let logs = HandlerLogs::events(&stderr, "handler.log");
        assert_eq!(logs.len(), 1, "{}: {stderr}", self.extension);
        let log = &logs[0];
        assert_eq!(log["level"], "info", "{}: {log}", self.extension);
        assert_eq!(log["message"], "slow upstream", "{}: {log}", self.extension);
        assert_eq!(
            log["fields"]["upstream_ms"], 812,
            "{}: {log}",
            self.extension
        );
        assert_eq!(log["truncated"], false, "{}: {log}", self.extension);
        assert_eq!(log["route"], "/logs", "{}: {log}", self.extension);
        assert_eq!(
            log["handler"].as_str(),
            Some(self.entry().as_str()),
            "{}: {log}",
            self.extension
        );
        assert!(
            log["request_id"].as_str().is_some_and(|id| !id.is_empty()),
            "{}: {log}",
            self.extension
        );
        assert!(
            !stderr.contains("hidden detail"),
            "{}: {stderr}",
            self.extension
        );
    }
}

struct HandlerLogs;

impl HandlerLogs {
    const READY_SECONDS: u64 = 60;

    fn write_start_files(project: &Path) {
        Self::write(&project.join("client/pages/tac.html"), "<test-view1 />");
        Self::write(
            &project.join("client/components/test/view1/tac.html"),
            "<main>Ready</main>",
        );
        Self::write(&project.join("dist/web/index.html"), "<main>Ready</main>");
    }

    fn origin(stdout: ChildStdout) -> String {
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let mut line = String::new();
            let _read = BufReader::new(stdout).read_line(&mut line);
            let _sent = sender.send(line);
        });
        let line = receiver
            .recv_timeout(Duration::from_secs(Self::READY_SECONDS))
            .expect("bounded readiness");
        let start = line.find("http://").expect("ready line names the origin");
        line[start..].trim().trim_end_matches('/').to_owned()
    }

    fn get(origin: &str, path: &str) -> String {
        let address = origin.trim_start_matches("http://");
        let mut stream = TcpStream::connect(address).expect("connect");
        stream
            .set_read_timeout(Some(Duration::from_secs(Self::READY_SECONDS)))
            .expect("read timeout");
        let request =
            format!("GET {path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n");
        stream.write_all(request.as_bytes()).expect("request");
        let mut answer = String::new();
        stream.read_to_string(&mut answer).expect("answer");
        answer
    }

    fn write(path: &Path, contents: &str) {
        fs::create_dir_all(path.parent().expect("fixture parent")).expect("fixture directory");
        fs::write(path, contents).expect("fixture source");
    }

    fn events(stderr: &str, name: &str) -> Vec<serde_json::Value> {
        stderr
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .filter(|event| event["event"] == name)
            .collect()
    }

    fn assert_answer(output: &Output) {
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let response: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("protocol response JSON");
        assert_eq!(response["status"], 200, "{response}");
        let data = response["body"]["data"].as_str().expect("body data");
        let body: serde_json::Value = serde_json::from_str(data).expect("JSON body");
        assert_eq!(body, serde_json::json!(["ok"]), "{response}");
    }
}

#[test]
fn a_javascript_handler_writes_a_log_line() {
    LogCase::JAVASCRIPT.assert_logs();
}

#[test]
fn a_typescript_handler_writes_a_log_line() {
    LogCase::TYPESCRIPT.assert_logs();
}

#[test]
fn a_python_handler_writes_a_log_line() {
    LogCase::PYTHON.assert_logs();
}

#[test]
fn a_ruby_handler_writes_a_log_line() {
    LogCase::RUBY.assert_logs();
}

#[test]
fn a_lua_handler_writes_a_log_line() {
    LogCase::LUA.assert_logs();
}

#[test]
fn a_php_handler_writes_a_log_line() {
    LogCase::PHP.assert_logs();
}

#[test]
fn a_java_handler_writes_a_log_line() {
    LogCase::JAVA.assert_logs();
}

#[test]
fn a_kotlin_handler_writes_a_log_line() {
    LogCase::KOTLIN.assert_logs();
}

#[test]
fn a_groovy_handler_writes_a_log_line() {
    LogCase::GROOVY.assert_logs();
}

#[test]
fn a_csharp_handler_writes_a_log_line() {
    LogCase::CSHARP.assert_logs();
}

#[test]
fn an_fsharp_handler_writes_a_log_line() {
    LogCase::FSHARP.assert_logs();
}

#[test]
fn a_rust_handler_writes_a_log_line() {
    LogCase::RUST.assert_logs();
}

#[test]
fn a_dart_handler_writes_a_log_line() {
    LogCase::DART.assert_logs();
}

#[test]
fn a_go_handler_writes_a_log_line() {
    LogCase::GO.assert_logs();
}

#[test]
fn a_cpp_handler_writes_a_log_line() {
    LogCase::CPP.assert_logs();
}

#[test]
fn the_level_setting_shows_debug_lines_and_off_hides_every_line() {
    let case = LogCase::JAVASCRIPT;
    if !case.available() {
        return;
    }
    let project = case.project();
    let debug = case.invoke(project.path(), Some("debug"));
    HandlerLogs::assert_answer(&debug);
    let shown = HandlerLogs::events(&String::from_utf8_lossy(&debug.stderr), "handler.log");
    let messages: Vec<&str> = shown
        .iter()
        .filter_map(|event| event["message"].as_str())
        .collect();
    assert_eq!(messages, ["hidden detail", "slow upstream"]);
    assert_eq!(shown[0]["level"], "debug");
    let off = case.invoke(project.path(), Some("off"));
    HandlerLogs::assert_answer(&off);
    let stderr = String::from_utf8_lossy(&off.stderr);
    assert!(
        HandlerLogs::events(&stderr, "handler.log").is_empty(),
        "{stderr}"
    );
}

#[test]
fn a_test_unit_writes_the_same_log_lines_under_ty_test() {
    let case = LogCase::JAVASCRIPT;
    if !case.available() {
        return;
    }
    let project = tempfile::tempdir().expect("project");
    HandlerLogs::write(
        &project.path().join("server/tests/LoggingTestSetup.js"),
        "@TestSetup()\nclass LoggingTestSetup {\n  static configure() { return {}; }\n}\n",
    );
    HandlerLogs::write(
        &project.path().join("server/tests/LoggingTest.js"),
        "@Test(LoggingTestSetup)\nexport default class LoggingTest {\n  async testWritesALogLine() {\n    YonLog.warn('from a test', { step: 1 })\n    Assert.equals(1, 1)\n  }\n}\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_ty"))
        .arg("test")
        .arg(project.path())
        .env_remove("TAC_HANDLER_LOG_LEVEL")
        .output()
        .expect("the ty process should start");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stdout}\n{stderr}");
    let logs = HandlerLogs::events(&stderr, "handler.log");
    assert_eq!(logs.len(), 1, "{stderr}");
    assert_eq!(logs[0]["level"], "warn");
    assert_eq!(logs[0]["message"], "from a test");
    assert_eq!(logs[0]["fields"]["step"], 1);
    assert_eq!(logs[0]["handler"], "server/tests/LoggingTest.js");
}

#[test]
fn ty_start_writes_log_lines_to_its_own_standard_error() {
    let case = LogCase::JAVASCRIPT;
    if !case.available() {
        return;
    }
    let project = case.project();
    HandlerLogs::write_start_files(project.path());
    let mut command = Command::new(env!("CARGO_BIN_EXE_ty"));
    command
        .arg("start")
        .arg(project.path())
        .args(["--host", "127.0.0.1", "--port", "0"])
        .env_remove("YON_PROFILE")
        .env_remove("TAC_HANDLER_LOG_LEVEL")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    WindowsEgress::proxied_where_unconfinable(&mut command);
    let mut server = command.spawn().expect("ty start");
    let origin = HandlerLogs::origin(server.stdout.take().expect("stdout"));
    let answer = HandlerLogs::get(&origin, "/logs");
    let _killed = server.kill();
    let _waited = server.wait();
    let mut stderr = String::new();
    server
        .stderr
        .take()
        .expect("stderr")
        .read_to_string(&mut stderr)
        .expect("server stderr");
    assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
    assert!(answer.ends_with("[\"ok\"]"), "{answer}");
    let logs = HandlerLogs::events(&stderr, "handler.log");
    assert_eq!(logs.len(), 1, "{stderr}");
    assert_eq!(logs[0]["message"], "slow upstream");
    assert_eq!(logs[0]["route"], "/logs");
    assert_eq!(logs[0]["handler"], "server/controllers/LogsController.js");
}
