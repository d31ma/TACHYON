#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;
use std::time::Duration;
use tachyon_contracts::{HandlerRequest, HandlerResponse, HttpMethod};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions, MigrationFix,
};

struct OldMiddleware {
    extension: &'static str,
    program: &'static str,
    version_flag: &'static str,
    source: &'static str,
}

impl OldMiddleware {
    const ALL: [Self; 9] = [
        Self {
            extension: "js",
            program: "node",
            version_flag: "--version",
            source: r"
export class Handler {
  static GET(request) {
    return new Response(null, { status: 204, headers: { 'x-phase': `seen-${request.method}` } })
  }

  static POST(request) {
    return new Response(null, { status: 204, headers: { 'x-phase': `seen-${request.method}` } })
  }
}
",
        },
        Self {
            extension: "ts",
            program: "bun",
            version_flag: "--version",
            source: r"
export class Handler {
  static GET(request: Request): Response {
    return new Response(null, { status: 204, headers: { 'x-phase': `seen-${request.method}` } })
  }

  static POST(request: Request): Response {
    return new Response(null, { status: 204, headers: { 'x-phase': `seen-${request.method}` } })
  }
}
",
        },
        Self {
            extension: "py",
            program: "python3",
            version_flag: "--version",
            source: r#"
class Handler:
    @staticmethod
    def GET(request):
        return (204, {"x-phase": "seen-" + request.get_method()}, "")

    @staticmethod
    def POST(request):
        return (204, {"x-phase": "seen-" + request.get_method()}, "")
"#,
        },
        Self {
            extension: "php",
            program: "php-cgi",
            version_flag: "--version",
            source: r"<?php
final class Handler
{
    public static function GET(): void
    {
        header('x-phase: seen-' . $_SERVER['REQUEST_METHOD']);
    }

    public static function POST(): void
    {
        header('x-phase: seen-' . $_SERVER['REQUEST_METHOD']);
    }
}
",
        },
        Self {
            extension: "java",
            program: "java",
            version_flag: "--version",
            source: r#"
class Handler {
    static void GET(com.sun.net.httpserver.HttpExchange exchange) throws java.io.IOException {
        exchange.getResponseHeaders().add("x-phase", "seen-" + exchange.getRequestMethod());
        exchange.sendResponseHeaders(204, -1);
    }

    static void POST(com.sun.net.httpserver.HttpExchange exchange) throws java.io.IOException {
        exchange.getResponseHeaders().add("x-phase", "seen-" + exchange.getRequestMethod());
        exchange.sendResponseHeaders(204, -1);
    }
}
"#,
        },
        Self {
            extension: "kt",
            program: "kotlinc",
            version_flag: "-version",
            source: r#"
object Handler {
    fun GET(exchange: com.sun.net.httpserver.HttpExchange) {
        exchange.responseHeaders.add("x-phase", "seen-" + exchange.requestMethod)
        exchange.sendResponseHeaders(204, -1)
    }

    fun POST(exchange: com.sun.net.httpserver.HttpExchange) {
        exchange.responseHeaders.add("x-phase", "seen-" + exchange.requestMethod)
        exchange.sendResponseHeaders(204, -1)
    }
}
"#,
        },
        Self {
            extension: "cs",
            program: "dotnet",
            version_flag: "--version",
            source: r#"
sealed class Handler {
    public static HttpResponseMessage GET(HttpRequestMessage request)
    {
        var response = new HttpResponseMessage(System.Net.HttpStatusCode.NoContent);
        response.Headers.Add("x-phase", "seen-" + request.Method.Method);
        return response;
    }

    public static HttpResponseMessage POST(HttpRequestMessage request)
    {
        var response = new HttpResponseMessage(System.Net.HttpStatusCode.NoContent);
        response.Headers.Add("x-phase", "seen-" + request.Method.Method);
        return response;
    }
}
"#,
        },
        Self {
            extension: "rs",
            program: "rustc",
            version_flag: "--version",
            source: r#"
struct Handler;
impl Handler {
    #[allow(non_snake_case)]
    fn GET(request: &http::Request<Vec<u8>>) -> http::Response<Vec<u8>> {
        let phase = format!("seen-{}", request.method());
        http::Response::builder().status(204).header("x-phase", phase).body(Vec::new()).unwrap_or_default()
    }

    #[allow(non_snake_case)]
    fn POST(request: &http::Request<Vec<u8>>) -> http::Response<Vec<u8>> {
        let phase = format!("seen-{}", request.method());
        http::Response::builder().status(204).header("x-phase", phase).body(Vec::new()).unwrap_or_default()
    }
}
"#,
        },
        Self {
            extension: "dart",
            program: "dart",
            version_flag: "--version",
            source: r"import 'package:shelf/shelf.dart';

class Handler {
  Response GET(Request request) =>
      Response(204, headers: {'x-phase': 'seen-${request.method}'});

  Response POST(Request request) =>
      Response(204, headers: {'x-phase': 'seen-${request.method}'});
}
",
        },
    ];

    fn is_available(&self) -> bool {
        let found =
            crate::support::program_probe::ProgramProbe::answers(self.program, self.version_flag);
        if !found {
            eprintln!(
                "skipping {}: {} is unavailable",
                self.extension, self.program
            );
        }
        found
    }

    fn migrated(&self) -> (tempfile::TempDir, HandlerSource) {
        let root = tempfile::tempdir().expect("project");
        let name = format!("middleware.{}", self.extension);
        fs::write(root.path().join(&name), self.source).expect("middleware source");
        let plan = MigrationFix::plan(root.path()).expect("plan");
        assert_eq!(plan.change_count(), 1, "{}", plan.to_text(false));
        plan.write(root.path()).expect("write");
        let again = MigrationFix::plan(root.path()).expect("second plan");
        assert_eq!(again.change_count(), 0, "{}", again.to_text(false));
        let source = HandlerSource::discover(root.path(), Path::new(&name))
            .unwrap_or_else(|failure| panic!("{}: {failure}", self.extension));
        (root, source)
    }

    fn supervisor(&self) -> HandlerSupervisor {
        let mut options = HandlerSupervisorOptions {
            default_timeout: Duration::from_mins(1),
            ..HandlerSupervisorOptions::default()
        };
        if self.extension == "ts" {
            options.runtimes.javascript = "bun".into();
        }
        HandlerSupervisor::new(options).expect("supervisor")
    }

    async fn run_phase(
        &self,
        source: &HandlerSource,
        operation: &str,
        method: HttpMethod,
    ) -> HandlerResponse {
        let mut request = HandlerRequest::route("middleware", "/orders", method);
        request.operation = String::from(operation);
        self.supervisor()
            .invoke(source, &request, &HandlerCancellation::default())
            .await
            .unwrap_or_else(|failure| panic!("{} {operation}: {failure}", self.extension))
    }
}

#[tokio::test]
async fn migrated_middleware_runs_its_old_code_in_both_phases_for_every_method() {
    for old in &OldMiddleware::ALL {
        if !old.is_available() {
            continue;
        }
        let (_root, source) = old.migrated();
        for (method, name) in [(HttpMethod::Options, "OPTIONS"), (HttpMethod::Post, "POST")] {
            for operation in ["middleware.before", "middleware.after"] {
                let response = old.run_phase(&source, operation, method).await;
                let label = format!("{} {operation} {name}", old.extension);
                assert!(response.error.is_none(), "{label}: {:?}", response.error);
                assert_eq!(response.status, 204, "{label}: {response:?}");
                let phase = response
                    .headers
                    .get("x-phase")
                    .and_then(|values| values.first());
                assert_eq!(
                    phase.map(String::as_str),
                    Some(format!("seen-{name}").as_str()),
                    "{label}"
                );
            }
        }
    }
}
