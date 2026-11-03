#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;
use std::time::Duration;
use tachyon_contracts::{HandlerRequest, HttpMethod};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
};

struct StopProject {
    extension: &'static str,
    program: &'static str,
    files: &'static [(&'static str, &'static str)],
}

impl StopProject {
    const MARKER: &'static str = "stop.marker";
    const STOPPED: &'static str = "stopped";
    const ANSWER: &'static str = "answered";

    const PYTHON: Self = Self {
        extension: "py",
        program: "python3",
        files: &[
            (
                "server/services/StopService.py",
                r#"@service
class StopService:
    def answer(self) -> str:
        return "answered"

    @on_stop
    def stop(self) -> None:
        with open("MARKERS/stop.marker", "w") as marker:
            marker.write("stopped")
"#,
            ),
            (
                "server/controllers/StopController.py",
                r#"from typing import Annotated


@request_route("/stop")
@public
class StopController:
    stop_service: Annotated[StopService, needs]

    @get_route
    def get_stop(self) -> str:
        return self.stop_service.answer()
"#,
            ),
        ],
    };

    const PHP: Self = Self {
        extension: "php",
        program: "php-cgi",
        files: &[
            (
                "server/services/StopService.php",
                r#"<?php
#[Service]
class StopService {
  function answer(): string {
    return "answered";
  }

  #[OnStop]
  function stop(): void {
    file_put_contents("MARKERS/stop.marker", "stopped");
  }
}
"#,
            ),
            (
                "server/controllers/StopController.php",
                r#"<?php
#[RequestRoute("/stop")]
#[Public]
class StopController {
  #[Needs]
  public StopService $stopService;
  #[GetRoute]
  function getStop(): string {
    return $this->stopService->answer();
  }
}
"#,
            ),
        ],
    };

    const JAVA: Self = Self {
        extension: "java",
        program: "java",
        files: &[
            (
                "server/services/StopService.java",
                r#"@Service
class StopService {
  String answer() {
    return "answered";
  }

  @OnStop
  void stop() {
    try {
      java.nio.file.Files.writeString(java.nio.file.Path.of("MARKERS/stop.marker"), "stopped");
    } catch (java.io.IOException failure) {
      throw new java.io.UncheckedIOException(failure);
    }
  }
}
"#,
            ),
            (
                "server/controllers/StopController.java",
                r#"@RequestRoute("/stop")
@Public
class StopController {
  @Needs
  final StopService stopService;
  @GetRoute
  String getStop() {
    return stopService.answer();
  }
}
"#,
            ),
        ],
    };

    const GROOVY: Self = Self {
        extension: "groovy",
        program: "groovyc",
        files: &[
            (
                "server/services/StopService.groovy",
                r#"@Service
class StopService {
  String answer() {
    return "answered"
  }

  @OnStop
  void stop() {
    new File("MARKERS/stop.marker").text = "stopped"
  }
}
"#,
            ),
            (
                "server/controllers/StopController.groovy",
                r#"@RequestRoute("/stop")
@Public
class StopController {
  @Needs
  final StopService stopService
  @GetRoute
  String getStop() {
    return stopService.answer()
  }
}
"#,
            ),
        ],
    };

    const KOTLIN: Self = Self {
        extension: "kt",
        program: "kotlinc",
        files: &[
            (
                "server/services/StopService.kt",
                r#"@Service
class StopService {
  fun answer(): String {
    return "answered"
  }

  @OnStop
  fun stop() {
    java.io.File("MARKERS/stop.marker").writeText("stopped")
  }
}
"#,
            ),
            (
                "server/controllers/StopController.kt",
                r#"@RequestRoute("/stop")
@Public
class StopController {
  @Needs
  lateinit var stopService: StopService
  @GetRoute
  fun getStop(): String {
    return stopService.answer()
  }
}
"#,
            ),
        ],
    };

    const RUST: Self = Self {
        extension: "rs",
        program: "rustc",
        files: &[
            (
                "server/services/StopService.rs",
                r#"#[Service]
struct StopService {}
impl StopService {
  fn new() -> Self { Self {} }

  fn answer(&self) -> String {
    String::from("answered")
  }

  #[OnStop]
  fn stop(&self) {
    std::fs::write("MARKERS/stop.marker", "stopped").expect("stop marker");
  }
}
"#,
            ),
            (
                "server/controllers/StopController.rs",
                r#"#[RequestRoute("/stop")]
#[Public]
struct StopController {
  #[Needs]
  stop_service: std::sync::Arc<StopService>,
}
impl StopController {
  #[GetRoute]
  fn get_stop(&self) -> String {
    self.stop_service.answer()
  }
}
"#,
            ),
        ],
    };

    const RUBY: Self = Self {
        extension: "rb",
        program: "ruby",
        files: &[
            (
                "server/services/StopService.rb",
                r#"#[Service]
class StopService
  # @return [String]
  def answer
    "answered"
  end

  # @return [void]
  #[OnStop]
  def stop
    File.write("MARKERS/stop.marker", "stopped")
  end
end
"#,
            ),
            (
                "server/controllers/StopController.rb",
                r#"#[RequestRoute("/stop")]
#[Public]
class StopController
  #[Needs]
  # @return [StopService]
  attr_reader :stop_service

  # @return [String]
  #[GetRoute]
  def get_stop
    stop_service.answer
  end
end
"#,
            ),
        ],
    };

    const TYPESCRIPT: Self = Self {
        extension: "ts",
        program: "node",
        files: &[
            (
                "server/services/StopService.ts",
                r"@Service
export default class StopService {
  answer(): string {
    return 'answered';
  }

  @OnStop
  stop(): void {
    writeFileSync('MARKERS/stop.marker', 'stopped');
  }
}
",
            ),
            (
                "server/controllers/StopController.ts",
                r#"import { writeFileSync } from 'node:fs';

@RequestRoute("/stop")
@Public
class StopController {
  @Needs
  readonly stopService!: StopService;
  @GetRoute
  getStop(): string {
    return this.stopService.answer();
  }
}
"#,
            ),
        ],
    };

    const CSHARP: Self = Self {
        extension: "cs",
        program: "dotnet",
        files: &[
            (
                "server/services/StopService.cs",
                r#"[Service]
class StopService {
  public string Answer() {
    return "answered";
  }

  [OnStop]
  public void Stop() {
    System.IO.File.WriteAllText("MARKERS/stop.marker", "stopped");
  }
}
"#,
            ),
            (
                "server/controllers/StopController.cs",
                r#"[RequestRoute("/stop")]
[Public]
class StopController {
  [Needs]
  readonly StopService _stopService;
  [GetRoute]
  public string GetStop() {
    return _stopService.Answer();
  }
}
"#,
            ),
        ],
    };

    const CPP: Self = Self {
        extension: "hpp",
        program: "clang++",
        files: &[
            (
                "server/services/StopService.hpp",
                r#"#pragma once
#include <fstream>
#include <string>

class [[Service]] StopService {
public:
    std::string answer() const { return "answered"; }

    [[OnStop]]
    void stop() {
        std::ofstream marker("MARKERS/stop.marker");
        marker << "stopped";
    }
};
"#,
            ),
            (
                "server/controllers/StopController.hpp",
                r#"#pragma once
#include <string>
#include "../services/StopService.hpp"

class [[RequestRoute("/stop")]] [[Public]] StopController {
    [[Needs]] StopService& stopService;

public:
    [[GetRoute]]
    std::string getStop() { return stopService.answer(); }
};
"#,
            ),
        ],
    };

    fn program(&self) -> String {
        if self.program == "node" {
            return std::env::var("YON_JAVASCRIPT_RUNTIME")
                .unwrap_or_else(|_| String::from("node"));
        }
        self.program.to_owned()
    }

    fn available(&self) -> bool {
        let program = self.program();
        let flag = match program.as_str() {
            "go" => "version",
            "kotlinc" => "-version",
            _ => "--version",
        };
        let found = crate::support::program_probe::ProgramProbe::answers(&program, flag);
        assert!(
            found || std::env::var_os("CI").is_none(),
            "{program} is missing in CI"
        );
        if !found {
            eprintln!("skipping .{}: {program} is unavailable", self.extension);
        }
        found
    }

    fn create(&self, root: &Path) {
        let markers = root.to_string_lossy().replace('\\', "/");
        for (relative, source) in self.files {
            Self::write(root, relative, &source.replace("MARKERS", &markers));
        }
    }

    fn write(root: &Path, relative: &str, source: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("directories");
        fs::write(path, source).expect("source");
    }

    fn describe(root: &Path, description: &str) {
        let relative = "server/services/StopService.hpp";
        let source = fs::read_to_string(root.join(relative)).expect("service");
        let marker = format!("[[Service, Description(\"{description}\")]]");
        Self::write(root, relative, &source.replace("[[Service]]", &marker));
    }

    fn discover(&self, root: &Path) -> HandlerSource {
        let entry = format!("server/controllers/StopController.{}", self.extension);
        HandlerSource::discover(root, &entry)
            .unwrap_or_else(|failure| panic!("{}: {failure}", self.extension))
    }

    async fn answer(&self, source: &HandlerSource) -> serde_json::Value {
        let supervisor =
            HandlerSupervisor::new(HandlerSupervisorOptions::default()).expect("supervisor");
        let request = HandlerRequest::route("stop_hooks", "/stop", HttpMethod::Get);
        let response = supervisor
            .invoke(source, &request, &HandlerCancellation::default())
            .await
            .unwrap_or_else(|failure| panic!("{}: {failure}", self.extension));
        assert_eq!(
            response.status, 200,
            "{}: {:?}",
            self.extension, response.error
        );
        let body = response.body.expect("body").data;
        serde_json::from_str(&body).unwrap_or_else(|_| panic!("{}: {body}", self.extension))
    }

    async fn written(marker: &Path) -> Option<String> {
        for _ in 0..50 {
            if let Ok(text) = fs::read_to_string(marker) {
                return Some(text);
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        None
    }

    async fn assert_stopped(&self) {
        if !self.available() {
            return;
        }
        let root = tempfile::tempdir().expect("project");
        self.create(root.path());
        self.assert_stops_in(root.path()).await;
    }

    async fn assert_stops_in(&self, root: &Path) {
        let marker = root.join(Self::MARKER);
        let source = self.discover(root);
        assert!(
            !marker.exists(),
            "{}: the stop marker exists before the request",
            self.extension
        );
        let answer = self.answer(&source).await;
        assert_eq!(
            answer,
            serde_json::json!(Self::ANSWER),
            "{}",
            self.extension
        );
        let written = Self::written(&marker).await;
        assert_eq!(
            written.as_deref(),
            Some(Self::STOPPED),
            "{}: the OnStop method did not run",
            self.extension
        );
    }
}

#[tokio::test]
async fn a_python_service_runs_its_on_stop_method_after_the_request() {
    StopProject::PYTHON.assert_stopped().await;
}

#[tokio::test]
async fn a_php_service_runs_its_on_stop_method_after_the_request() {
    StopProject::PHP.assert_stopped().await;
}

#[tokio::test]
async fn a_java_service_runs_its_on_stop_method_after_the_request() {
    StopProject::JAVA.assert_stopped().await;
}

#[tokio::test]
async fn a_groovy_service_runs_its_on_stop_method_after_the_request() {
    StopProject::GROOVY.assert_stopped().await;
}

#[tokio::test]
async fn a_kotlin_service_runs_its_on_stop_method_after_the_request() {
    StopProject::KOTLIN.assert_stopped().await;
}

#[tokio::test]
async fn a_rust_service_runs_its_on_stop_method_after_the_request() {
    StopProject::RUST.assert_stopped().await;
}

#[tokio::test]
async fn a_ruby_service_runs_its_on_stop_method_after_the_request() {
    StopProject::RUBY.assert_stopped().await;
}

#[tokio::test]
async fn a_typescript_service_runs_its_on_stop_method_after_the_request() {
    StopProject::TYPESCRIPT.assert_stopped().await;
}

#[tokio::test]
async fn a_csharp_service_runs_its_on_stop_method_after_the_request() {
    StopProject::CSHARP.assert_stopped().await;
}

#[tokio::test]
async fn a_cpp_service_runs_its_on_stop_method_after_the_request() {
    StopProject::CPP.assert_stopped().await;
}

#[tokio::test]
async fn a_cpp_description_changes_nothing_that_the_service_does() {
    let project = StopProject::CPP;
    if !project.available() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    project.create(root.path());
    StopProject::describe(root.path(), "Writes the stop marker");
    project.assert_stops_in(root.path()).await;
}

#[test]
fn the_yon_cpp_build_checks_a_description() {
    for description in ["", "Writes the \\\"stop\\\" marker"] {
        let root = tempfile::tempdir().expect("project");
        StopProject::CPP.create(root.path());
        StopProject::describe(root.path(), description);
        let entry = "server/controllers/StopController.hpp";
        let Err(refused) = HandlerSource::discover(root.path(), entry) else {
            panic!("the description {description} was accepted");
        };
        let refused = refused.to_string();
        assert!(
            refused.contains("[[Description]] takes one quoted line"),
            "{description}: {refused}"
        );
    }
}
