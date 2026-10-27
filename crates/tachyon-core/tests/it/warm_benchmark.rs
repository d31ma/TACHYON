#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};
use tachyon_contracts::{HandlerRequest, HttpMethod};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions, WarmSettings,
    YonIsolationPolicy,
};

struct WarmBenchmark {
    language: &'static str,
    path: &'static str,
    files: &'static [(&'static str, &'static str)],
}

impl WarmBenchmark {
    const LATER_REQUESTS: usize = 50;
    const ROUTE: &'static str = "/answer";
    const HANDLERS: [Self; 15] = [
        Self {
            language: "JavaScript",
            path: "server/workers/answer.js",
            files: &[(
                "server/workers/answer.js",
                "export class Handler {\n  static async POST(request) {\n    return { answer: 42 }\n  }\n}\n",
            )],
        },
        Self {
            language: "TypeScript",
            path: "server/workers/answer.ts",
            files: &[(
                "server/workers/answer.ts",
                "export class Handler {\n  static async POST(request: Request): Promise<object> {\n    return { answer: 42 }\n  }\n}\n",
            )],
        },
        Self {
            language: "Python",
            path: "server/workers/answer.py",
            files: &[(
                "server/workers/answer.py",
                "class Handler:\n    @staticmethod\n    def POST(request):\n        return {\"answer\": 42}\n",
            )],
        },
        Self {
            language: "Ruby",
            path: "server/workers/answer.rb",
            files: &[(
                "server/workers/answer.rb",
                "class Handler\n  def self.POST(request)\n    { \"answer\" => 42 }\n  end\nend\n",
            )],
        },
        Self {
            language: "PHP",
            path: "server/workers/answer.php",
            files: &[(
                "server/workers/answer.php",
                "<?php\nfinal class Handler\n{\n    public static function POST(): array\n    {\n        return ['answer' => 42];\n    }\n}\n",
            )],
        },
        Self {
            language: "Lua",
            path: "server/workers/answer.lua",
            files: &[(
                "server/workers/answer.lua",
                "---@class Handler\nHandler = {}\n\n---@param request table\n---@return table\nfunction Handler.POST(request)\n  return { answer = 42 }\nend\n",
            )],
        },
        Self {
            language: "Go",
            path: "server/workers/answer.go",
            files: &[(
                "server/workers/answer.go",
                "import \"net/http\"\n\ntype Handler struct{}\n\nfunc (Handler) POST(request *http.Request) any {\n\treturn map[string]int{\"answer\": 42}\n}\n",
            )],
        },
        Self {
            language: "Rust",
            path: "server/workers/answer.rs",
            files: &[(
                "server/workers/answer.rs",
                "struct Handler;\nimpl Handler {\n    fn POST(request: &__YonFrame) -> Json {\n        let size = request.body().len() as u64;\n        Json::map([(\"answer\", 42), (\"size\", size)])\n    }\n}\n",
            )],
        },
        Self {
            language: "C++",
            path: "server/controllers/AnswerController.hpp",
            files: &[
                (
                    "server/models/Answer.hpp",
                    "#pragma once\n\nstruct Answer {\n    int answer = 0;\n};\n",
                ),
                (
                    "server/controllers/AnswerController.hpp",
                    "#pragma once\n#include \"../models/Answer.hpp\"\n\nclass [[RequestRoute(\"/answer\")]] [[Public]] AnswerController {\npublic:\n    [[PostRoute]]\n    Answer postAnswer() {\n        return Answer{42};\n    }\n};\n",
                ),
            ],
        },
        Self {
            language: "Kotlin",
            path: "server/workers/answer.kt",
            files: &[(
                "server/workers/answer.kt",
                "object Handler {\n    @JvmStatic fun POST(): Any? = mapOf(\"answer\" to 42)\n}\n",
            )],
        },
        Self {
            language: "C#",
            path: "server/workers/answer.cs",
            files: &[(
                "server/workers/answer.cs",
                "sealed class Handler\n{\n    public static object POST() => new { answer = 42 };\n}\n",
            )],
        },
        Self {
            language: "F#",
            path: "server/workers/answer.fs",
            files: &[(
                "server/workers/answer.fs",
                "type Handler() =\n    static member POST() : obj = box {| answer = 42 |}\n",
            )],
        },
        Self {
            language: "Dart",
            path: "server/workers/answer.dart",
            files: &[(
                "server/workers/answer.dart",
                "class Handler {\n  Object? POST() => {'answer': 42};\n}\n",
            )],
        },
        Self {
            language: "Java",
            path: "server/workers/answer.java",
            files: &[(
                "server/workers/answer.java",
                "class Handler {\n    static Object POST() {\n        return java.util.Map.of(\"answer\", 42);\n    }\n}\n",
            )],
        },
        Self {
            language: "Groovy",
            path: "server/workers/answer.groovy",
            files: &[(
                "server/workers/answer.groovy",
                "class Handler {\n    static Object POST() {\n        return [answer: 42]\n    }\n}\n",
            )],
        },
    ];

    fn project(&self) -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        for (relative, contents) in self.files {
            Self::write(root.path(), relative, contents);
        }
        root
    }

    fn write(root: &Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("handler parent")).expect("handler directory");
        fs::write(path, contents).expect("handler source");
    }

    fn supervisor(isolation: YonIsolationPolicy) -> HandlerSupervisor {
        HandlerSupervisor::new(HandlerSupervisorOptions {
            isolation,
            default_timeout: Duration::from_mins(5),
            ..HandlerSupervisorOptions::default()
        })
        .expect("benchmark supervisor")
    }

    fn warm_mode() -> YonIsolationPolicy {
        YonIsolationPolicy::Warm(WarmSettings::new(1_000_000, Duration::from_mins(10)))
    }

    async fn timed(
        supervisor: &HandlerSupervisor,
        source: &HandlerSource,
    ) -> Result<Duration, String> {
        let request = HandlerRequest::route("benchmark", Self::ROUTE, HttpMethod::Post);
        let started = Instant::now();
        let response = supervisor
            .invoke(source, &request, &HandlerCancellation::default())
            .await
            .map_err(|failure| format!("{failure:?}"))?;
        let elapsed = started.elapsed();
        if let Some(error) = response.error {
            return Err(format!("{error:?}"));
        }
        let body = response.body.map(|body| body.data).unwrap_or_default();
        if !body.contains("42") {
            return Err(format!("unexpected body {body}"));
        }
        Ok(elapsed)
    }

    async fn measured(
        source: &HandlerSource,
        isolation: YonIsolationPolicy,
    ) -> Result<String, String> {
        let supervisor = Self::supervisor(isolation);
        let first = Self::timed(&supervisor, source).await?;
        let mut later = Vec::with_capacity(Self::LATER_REQUESTS);
        for _ in 0..Self::LATER_REQUESTS {
            later.push(Self::timed(&supervisor, source).await?);
        }
        drop(supervisor);
        later.sort();
        let median = later[later.len() / 2];
        let p95_index = (later.len() * 95).div_ceil(100) - 1;
        let p95 = later[p95_index];
        Ok(format!(
            "{} | {} | {}",
            Self::milliseconds(first),
            Self::milliseconds(median),
            Self::milliseconds(p95)
        ))
    }

    fn milliseconds(duration: Duration) -> String {
        format!("{:.1}", duration.as_secs_f64() * 1000.0)
    }

    async fn row(&self) -> String {
        let root = self.project();
        let source = match HandlerSource::discover(root.path(), self.path) {
            Ok(source) => source,
            Err(failure) => return format!("| {} | skipped: {failure:?} |", self.language),
        };
        let prime = Self::supervisor(YonIsolationPolicy::Process);
        let primed = Self::timed(&prime, &source).await;
        drop(prime);
        let compile = match primed {
            Ok(duration) => Self::milliseconds(duration),
            Err(reason) => return format!("| {} | skipped: {reason} |", self.language),
        };
        let warm = Self::measured(&source, Self::warm_mode()).await;
        let process = Self::measured(&source, YonIsolationPolicy::Process).await;
        let warm = warm.unwrap_or_else(|reason| format!("failed: {reason} | | "));
        let process = process.unwrap_or_else(|reason| format!("failed: {reason} | | "));
        format!("| {} | {compile} | {warm} | {process} |", self.language)
    }

    fn header() -> String {
        [
            "| Language | Prime (ms) | Warm first (ms) | Warm median (ms) | Warm p95 (ms) | Process first (ms) | Process median (ms) | Process p95 (ms) |",
            "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
        ]
        .join("\n")
    }
}

#[tokio::test]
#[ignore = "benchmark: run with --ignored --nocapture"]
async fn warm_and_process_latency_for_each_handler_language() {
    let only = std::env::var("YON_BENCHMARK_LANGUAGES").unwrap_or_default();
    println!("{}", WarmBenchmark::header());
    for benchmark in &WarmBenchmark::HANDLERS {
        let chosen = only.is_empty() || only.split(',').any(|name| name == benchmark.language);
        if chosen {
            println!("{}", benchmark.row().await);
        }
    }
}
