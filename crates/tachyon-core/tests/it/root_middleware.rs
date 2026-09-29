#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;
use tachyon_contracts::{HandlerRequest, HandlerResponse, HttpMethod};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
};

struct Language {
    extension: &'static str,
    program: &'static str,
    version_flag: &'static str,
    both_phases: &'static str,
    after_only: &'static str,
}

impl Language {
    const ALL: [Self; 13] = [
        Self {
            extension: "js",
            program: "node",
            version_flag: "--version",
            both_phases: r"
export class Middleware {
  static before(request) {
    return new Response(null, { status: 204, headers: { 'x-phase': `before-${request.method}` } })
  }

  static after(request) {
    return new Response(null, { status: 204, headers: { 'x-phase': `after-${request.method}` } })
  }
}
",
            after_only: r"
export class Middleware {
  static after(request) {
    return new Response(null, { status: 204, headers: { 'x-phase': 'after' } })
  }
}
",
        },
        Self {
            extension: "ts",
            program: "bun",
            version_flag: "--version",
            both_phases: r"
export class Middleware {
  static before(request: Request): Response {
    return new Response(null, { status: 204, headers: { 'x-phase': `before-${request.method}` } })
  }

  static after(request: Request): Response {
    return new Response(null, { status: 204, headers: { 'x-phase': `after-${request.method}` } })
  }
}
",
            after_only: r"
export class Middleware {
  static after(request: Request): Response {
    return new Response(null, { status: 204, headers: { 'x-phase': 'after' } })
  }
}
",
        },
        Self {
            extension: "py",
            program: "python3",
            version_flag: "--version",
            both_phases: r#"
from http import HTTPStatus


class Middleware:
    @staticmethod
    def before(request):
        return (HTTPStatus.NO_CONTENT, {"x-phase": "before-" + request.get_method()}, "")

    @staticmethod
    def after(request):
        return (HTTPStatus.NO_CONTENT, {"x-phase": "after-" + request.get_method()}, "")
"#,
            after_only: r#"
from http import HTTPStatus


class Middleware:
    @staticmethod
    def after(request):
        return (HTTPStatus.NO_CONTENT, {"x-phase": "after"}, "")
"#,
        },
        Self {
            extension: "php",
            program: "php-cgi",
            version_flag: "--version",
            both_phases: r"<?php
final class Middleware
{
    public static function before(): void
    {
        header('x-phase: before-' . $_SERVER['REQUEST_METHOD']);
    }

    public static function after(): void
    {
        header('x-phase: after-' . $_SERVER['REQUEST_METHOD']);
    }
}
",
            after_only: r"<?php
final class Middleware
{
    public static function after(): void
    {
        header('x-phase: after');
    }
}
",
        },
        Self {
            extension: "java",
            program: "java",
            version_flag: "--version",
            both_phases: r#"
class Middleware {
    static void before(com.sun.net.httpserver.HttpExchange exchange) throws java.io.IOException {
        exchange.getResponseHeaders().add("x-phase", "before-" + exchange.getRequestMethod());
        exchange.sendResponseHeaders(204, -1);
    }

    static void after(com.sun.net.httpserver.HttpExchange exchange) throws java.io.IOException {
        exchange.getResponseHeaders().add("x-phase", "after-" + exchange.getRequestMethod());
        exchange.sendResponseHeaders(204, -1);
    }
}
"#,
            after_only: r#"
class Middleware {
    static void after(com.sun.net.httpserver.HttpExchange exchange) throws java.io.IOException {
        exchange.getResponseHeaders().add("x-phase", "after");
        exchange.sendResponseHeaders(204, -1);
    }
}
"#,
        },
        Self {
            extension: "groovy",
            program: "groovyc",
            version_flag: "--version",
            both_phases: r#"
import com.sun.net.httpserver.HttpExchange

class Middleware {
    static void before(HttpExchange exchange) {
        exchange.responseHeaders.add('x-phase', "before-${exchange.requestMethod}")
        exchange.sendResponseHeaders(204, -1)
    }

    static void after(HttpExchange exchange) {
        exchange.responseHeaders.add('x-phase', "after-${exchange.requestMethod}")
        exchange.sendResponseHeaders(204, -1)
    }
}
"#,
            after_only: r"
import com.sun.net.httpserver.HttpExchange

class Middleware {
    static void after(HttpExchange exchange) {
        exchange.responseHeaders.add('x-phase', 'after')
        exchange.sendResponseHeaders(204, -1)
    }
}
",
        },
        Self {
            extension: "kt",
            program: "kotlinc",
            version_flag: "-version",
            both_phases: r#"
import com.sun.net.httpserver.HttpExchange

object Middleware {
    fun before(exchange: HttpExchange) {
        exchange.responseHeaders.add("x-phase", "before-" + exchange.requestMethod)
        exchange.sendResponseHeaders(204, -1)
    }

    fun after(exchange: HttpExchange) {
        exchange.responseHeaders.add("x-phase", "after-" + exchange.requestMethod)
        exchange.sendResponseHeaders(204, -1)
    }
}
"#,
            after_only: r#"
import com.sun.net.httpserver.HttpExchange

object Middleware {
    fun after(exchange: HttpExchange) {
        exchange.responseHeaders.add("x-phase", "after")
        exchange.sendResponseHeaders(204, -1)
    }
}
"#,
        },
        Self {
            extension: "cs",
            program: "dotnet",
            version_flag: "--version",
            both_phases: r#"
sealed class Middleware {
    public static HttpResponseMessage before(HttpRequestMessage request) => Phase("before-" + request.Method.Method);

    public static HttpResponseMessage after(HttpRequestMessage request) => Phase("after-" + request.Method.Method);

    static HttpResponseMessage Phase(string value) {
        var response = new HttpResponseMessage(System.Net.HttpStatusCode.NoContent);
        response.Headers.Add("x-phase", value);
        return response;
    }
}
"#,
            after_only: r#"
sealed class Middleware {
    public static HttpResponseMessage after(HttpRequestMessage request) {
        var response = new HttpResponseMessage(System.Net.HttpStatusCode.NoContent);
        response.Headers.Add("x-phase", "after");
        return response;
    }
}
"#,
        },
        Self {
            extension: "rs",
            program: "rustc",
            version_flag: "--version",
            both_phases: r#"
struct Middleware;
impl Middleware {
    fn before(request: &http::Request<Vec<u8>>) -> http::Response<Vec<u8>> {
        Self::phase(&format!("before-{}", request.method()))
    }

    fn after(request: &http::Request<Vec<u8>>) -> http::Response<Vec<u8>> {
        Self::phase(&format!("after-{}", request.method()))
    }

    fn phase(value: &str) -> http::Response<Vec<u8>> {
        http::Response::builder()
            .status(204)
            .header("x-phase", value)
            .body(Vec::new())
            .unwrap_or_default()
    }
}
"#,
            after_only: r#"
struct Middleware;
impl Middleware {
    fn after(_request: &http::Request<Vec<u8>>) -> http::Response<Vec<u8>> {
        http::Response::builder()
            .status(204)
            .header("x-phase", "after")
            .body(Vec::new())
            .unwrap_or_default()
    }
}
"#,
        },
        Self {
            extension: "dart",
            program: "dart",
            version_flag: "--version",
            both_phases: r"
import 'package:shelf/shelf.dart';

class Middleware {
  Response before(Request request) =>
      Response(204, headers: {'x-phase': 'before-${request.method}'});

  Response after(Request request) =>
      Response(204, headers: {'x-phase': 'after-${request.method}'});
}
",
            after_only: r"
import 'package:shelf/shelf.dart';

class Middleware {
  Response after(Request request) => Response(204, headers: {'x-phase': 'after'});
}
",
        },
        Self {
            extension: "go",
            program: "go",
            version_flag: "version",
            both_phases: r#"
import "net/http"

type Middleware struct{}

func (Middleware) Before(request *http.Request) *http.Response {
	header := http.Header{"X-Phase": {"before-" + request.Method}}
	return &http.Response{StatusCode: http.StatusNoContent, Header: header}
}

func (Middleware) After(request *http.Request) *http.Response {
	header := http.Header{"X-Phase": {"after-" + request.Method}}
	return &http.Response{StatusCode: http.StatusNoContent, Header: header}
}
"#,
            after_only: r#"
import "net/http"

type Middleware struct{}

func (Middleware) After(request *http.Request) *http.Response {
	return &http.Response{StatusCode: http.StatusNoContent, Header: http.Header{"X-Phase": {"after"}}}
}
"#,
        },
        Self {
            extension: "lua",
            program: "lua",
            version_flag: "-v",
            both_phases: r#"
Middleware = {}

function Middleware.before(request)
  return { 204, { ["x-phase"] = "before-" .. request.method }, {} }
end

function Middleware.after(request)
  return { 204, { ["x-phase"] = "after-" .. request.method }, {} }
end
"#,
            after_only: r#"
Middleware = {}

function Middleware.after(request)
  return { 204, { ["x-phase"] = "after" }, {} }
end
"#,
        },
        Self {
            extension: "rb",
            program: "ruby",
            version_flag: "--version",
            both_phases: r#"
class Middleware
  def self.before(request)
    [204, { "x-phase" => "before-#{request.method}" }, []]
  end

  def self.after(request)
    [204, { "x-phase" => "after-#{request.method}" }, []]
  end
end
"#,
            after_only: r#"
class Middleware
  def self.after(request)
    [204, { "x-phase" => "after" }, []]
  end
end
"#,
        },
    ];

    fn is_available(&self) -> bool {
        Command::new(self.program)
            .arg(self.version_flag)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }

    fn is_usable(&self) -> bool {
        if self.is_available() {
            return true;
        }
        let required =
            matches!(self.extension, "go" | "rb") && cfg!(unix) && std::env::var_os("CI").is_some();
        assert!(
            !required,
            "{} is required for the middleware tests in CI",
            self.program
        );
        eprintln!(
            "skipping {}: {} is unavailable",
            self.extension, self.program
        );
        false
    }

    fn middleware(&self, contents: &str) -> (tempfile::TempDir, HandlerSource) {
        let root = tempfile::tempdir().expect("project");
        let name = format!("middleware.{}", self.extension);
        fs::write(root.path().join(&name), contents).expect("middleware source");
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
}

async fn run_phase(
    language: &Language,
    source: &HandlerSource,
    operation: &str,
    method: HttpMethod,
) -> HandlerResponse {
    let mut request = HandlerRequest::route("middleware", "/orders", method);
    request.operation = String::from(operation);
    language
        .supervisor()
        .invoke(source, &request, &HandlerCancellation::default())
        .await
        .unwrap_or_else(|failure| panic!("{} {operation}: {failure}", language.extension))
}

fn phase_header(response: &HandlerResponse) -> Option<&str> {
    let values = response.headers.get("x-phase")?;
    values.first().map(String::as_str)
}

#[tokio::test]
async fn every_language_calls_before_and_after_for_every_http_method() {
    for language in &Language::ALL {
        if !language.is_usable() {
            continue;
        }
        let (_root, source) = language.middleware(language.both_phases);
        for (method, name) in [(HttpMethod::Options, "OPTIONS"), (HttpMethod::Post, "POST")] {
            for (operation, phase) in [
                ("middleware.before", "before"),
                ("middleware.after", "after"),
            ] {
                let response = run_phase(language, &source, operation, method).await;
                let label = format!("{} {operation} {name}", language.extension);
                assert!(response.error.is_none(), "{label}: {:?}", response.error);
                assert_eq!(response.status, 204, "{label}: {response:?}");
                let expected = format!("{phase}-{name}");
                assert_eq!(phase_header(&response), Some(expected.as_str()), "{label}");
            }
        }
    }
}

#[tokio::test]
async fn a_missing_phase_passes_the_request_through() {
    for language in &Language::ALL {
        if !language.is_usable() {
            continue;
        }
        let (_root, source) = language.middleware(language.after_only);
        let label = language.extension;
        let before = run_phase(language, &source, "middleware.before", HttpMethod::Delete).await;
        assert!(before.error.is_none(), "{label}: {:?}", before.error);
        assert_eq!(before.status, 204, "{label}: {before:?}");
        assert_eq!(phase_header(&before), None, "{label}");
        let after = run_phase(language, &source, "middleware.after", HttpMethod::Delete).await;
        assert_eq!(after.status, 204, "{label}: {after:?}");
        assert_eq!(phase_header(&after), Some("after"), "{label}");
    }
}

struct NativeRequest;

impl NativeRequest {
    const JAVASCRIPT: &str = r"
export class Middleware {
  static before(request) {
    const url = new URL(request.url)
    if (url.pathname.startsWith('/private') && request.headers.get('authorization') !== 'let-me-in') {
      return Response.json({ error: 'Unauthorized', query: url.searchParams.get('page') }, { status: 401 })
    }
  }
}
";

    const PYTHON: &str = r#"
import json
import urllib.parse
from http import HTTPStatus


class Middleware:
    @staticmethod
    def before(request):
        url = urllib.parse.urlsplit(request.full_url)
        if url.path.startswith("/private") and request.get_header("Authorization") != "let-me-in":
            page = urllib.parse.parse_qs(url.query)["page"][0]
            body = json.dumps({"error": "Unauthorized", "query": page}, separators=(",", ":"))
            return (HTTPStatus.UNAUTHORIZED, {"content-type": "application/json"}, body)
        return None
"#;

    const RUBY: &str = r#"
class Middleware
  def self.before(request)
    uri = URI(request.path)
    return nil if !uri.path.start_with?("/private") || request["authorization"] == "let-me-in"

    page = URI.decode_www_form(uri.query).to_h["page"]
    [401, { "content-type" => "application/json" }, [JSON.generate({ "error" => "Unauthorized", "query" => page })]]
  end
end
"#;

    const GO: &str = r#"
import (
	"encoding/json"
	"io"
	"net/http"
	"strings"
)

type Middleware struct{}

func (Middleware) Before(request *http.Request) *http.Response {
	if !strings.HasPrefix(request.URL.Path, "/private") || request.Header.Get("Authorization") == "let-me-in" {
		return nil
	}
	body, _ := json.Marshal(map[string]string{"error": "Unauthorized", "query": request.URL.Query().Get("page")})
	header := http.Header{"Content-Type": {"application/json"}}
	return &http.Response{StatusCode: http.StatusUnauthorized, Header: header, Body: io.NopCloser(strings.NewReader(string(body)))}
}
"#;

    const JAVA: &str = r#"
import com.sun.net.httpserver.HttpExchange;

class Middleware {
    static void before(HttpExchange exchange) throws java.io.IOException {
        String path = exchange.getRequestURI().getPath();
        if (!path.startsWith("/private") || "let-me-in".equals(exchange.getRequestHeaders().getFirst("Authorization"))) return;
        String page = exchange.getRequestURI().getQuery().replace("page=", "");
        byte[] body = ("{\"error\":\"Unauthorized\",\"query\":\"" + page + "\"}").getBytes(java.nio.charset.StandardCharsets.UTF_8);
        exchange.getResponseHeaders().add("content-type", "application/json");
        exchange.sendResponseHeaders(401, body.length);
        exchange.getResponseBody().write(body);
    }
}
"#;

    const GROOVY: &str = r#"
import com.sun.net.httpserver.HttpExchange
import java.nio.charset.StandardCharsets

class Middleware {
    static void before(HttpExchange exchange) {
        String path = exchange.requestURI.path
        if (!path.startsWith('/private') || exchange.requestHeaders.getFirst('Authorization') == 'let-me-in') {
            return
        }
        String page = exchange.requestURI.query.replace('page=', '')
        byte[] body = """{"error":"Unauthorized","query":"${page}"}""".getBytes(StandardCharsets.UTF_8)
        exchange.responseHeaders.add('content-type', 'application/json')
        exchange.sendResponseHeaders(401, body.length)
        exchange.responseBody.write(body)
    }
}
"#;

    const CSHARP: &str = r#"
sealed class Middleware {
    public static HttpResponseMessage before(HttpRequestMessage request) {
        var path = request.RequestUri.AbsolutePath;
        var token = request.Headers.Authorization?.ToString();
        if (!path.StartsWith("/private") || token == "let-me-in") return null;
        var page = System.Web.HttpUtility.ParseQueryString(request.RequestUri.Query)["page"];
        var body = System.Text.Json.JsonSerializer.Serialize(new { error = "Unauthorized", query = page });
        return new HttpResponseMessage(System.Net.HttpStatusCode.Unauthorized) { Content = new StringContent(body, System.Text.Encoding.UTF8, "application/json") };
    }
}
"#;

    const LUA: &str = r#"
Middleware = {}

function Middleware.before(request)
  local path = request.path:match("^[^?]*")
  if path:sub(1, 8) ~= "/private" or request:header("authorization") == "let-me-in" then
    return nil
  end
  local body = '{"error":"Unauthorized","query":"' .. request:query("page") .. '"}'
  return { 401, { ["content-type"] = "application/json" }, { body } }
end
"#;

    const DART: &str = r"
import 'dart:convert';
import 'package:shelf/shelf.dart';

class Middleware {
  Response? before(Request request) {
    final uri = request.requestedUri;
    if (!uri.path.startsWith('/private') || request.headers['authorization'] == 'let-me-in') return null;
    final body = jsonEncode({'error': 'Unauthorized', 'query': uri.queryParameters['page']});
    return Response(401, body: body, headers: {'content-type': 'application/json'});
  }
}
";

    const RUST: &str = r#"
struct Middleware;
impl Middleware {
    fn before(request: &http::Request<Vec<u8>>) -> Option<http::Response<Vec<u8>>> {
        let path = request.uri().path();
        let token = request.headers().get("authorization").and_then(|value| value.to_str().ok());
        if !path.starts_with("/private") || token == Some("let-me-in") {
            return None;
        }
        let page = request.uri().query().unwrap_or_default().trim_start_matches("page=");
        let body = format!("{{\"error\":\"Unauthorized\",\"query\":\"{page}\"}}");
        http::Response::builder()
            .status(401)
            .header("content-type", "application/json")
            .body(body.into_bytes())
            .ok()
    }
}
"#;

    const PHP: &str = r"<?php
final class Middleware
{
    public static function before(): void
    {
        $path = parse_url($_SERVER['REQUEST_URI'], PHP_URL_PATH);
        if (!str_starts_with($path, '/private') || ($_SERVER['HTTP_AUTHORIZATION'] ?? '') === 'let-me-in') {
            return;
        }
        http_response_code(401);
        header('Content-Type: application/json');
        echo json_encode(['error' => 'Unauthorized', 'query' => $_GET['page']]);
    }
}
";

    fn language(extension: &str) -> Option<&'static Language> {
        let language = Language::ALL
            .iter()
            .find(|language| language.extension == extension)?;
        language.is_usable().then_some(language)
    }

    async fn check(extension: &str, contents: &str) {
        let Some(language) = Self::language(extension) else {
            return;
        };
        let refused = Self::answer(language, contents, None).await;
        assert_eq!(refused.status, 401, "{extension}: {refused:?}");
        let body = refused.body.expect("body").data;
        assert_eq!(
            body, r#"{"error":"Unauthorized","query":"2"}"#,
            "{extension}"
        );
        let allowed = Self::answer(language, contents, Some("let-me-in")).await;
        assert_eq!(allowed.status, 204, "{extension}: {allowed:?}");
    }

    async fn answer(
        language: &Language,
        contents: &str,
        authorization: Option<&str>,
    ) -> HandlerResponse {
        let (_root, source) = language.middleware(contents);
        let mut request = HandlerRequest::route("native", "/private/orders", HttpMethod::Get);
        request.operation = String::from("middleware.before");
        request.target = Some(String::from("/private/orders?page=2"));
        if let Some(value) = authorization {
            request
                .headers
                .insert(String::from("authorization"), vec![String::from(value)]);
        }
        language
            .supervisor()
            .invoke(&source, &request, &HandlerCancellation::default())
            .await
            .unwrap_or_else(|failure| panic!("{}: {failure}", language.extension))
    }
}

#[tokio::test]
async fn javascript_middleware_reads_a_fetch_request_and_answers_with_a_fetch_response() {
    NativeRequest::check("js", NativeRequest::JAVASCRIPT).await;
}

#[tokio::test]
async fn python_middleware_reads_a_urllib_request_and_answers_with_a_status_tuple() {
    NativeRequest::check("py", NativeRequest::PYTHON).await;
}

#[tokio::test]
async fn ruby_middleware_reads_a_net_http_request_and_answers_with_a_rack_triplet() {
    NativeRequest::check("rb", NativeRequest::RUBY).await;
}

#[tokio::test]
async fn lua_middleware_reads_a_request_table_and_answers_with_a_status_list() {
    NativeRequest::check("lua", NativeRequest::LUA).await;
}

#[tokio::test]
async fn go_middleware_reads_a_net_http_request_and_answers_with_a_net_http_response() {
    NativeRequest::check("go", NativeRequest::GO).await;
}

#[tokio::test]
async fn java_middleware_reads_an_http_exchange_and_answers_on_it() {
    NativeRequest::check("java", NativeRequest::JAVA).await;
}

#[tokio::test]
async fn groovy_middleware_reads_an_http_exchange_and_answers_on_it() {
    NativeRequest::check("groovy", NativeRequest::GROOVY).await;
}

#[tokio::test]
async fn csharp_middleware_reads_an_http_request_message_and_answers_with_an_http_response_message()
{
    NativeRequest::check("cs", NativeRequest::CSHARP).await;
}

#[tokio::test]
async fn dart_middleware_reads_a_shelf_request_and_answers_with_a_shelf_response() {
    NativeRequest::check("dart", NativeRequest::DART).await;
}

#[tokio::test]
async fn rust_middleware_reads_an_http_request_and_answers_with_an_http_response() {
    NativeRequest::check("rs", NativeRequest::RUST).await;
}

#[tokio::test]
async fn php_middleware_reads_the_superglobals_and_answers_with_native_calls() {
    NativeRequest::check("php", NativeRequest::PHP).await;
}
