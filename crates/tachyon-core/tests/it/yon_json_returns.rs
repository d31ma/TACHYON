#![allow(clippy::expect_used)]

use std::process::{Command, Stdio};
use std::time::Duration;
use tachyon_contracts::{HandlerRequest, HandlerResponse, HttpMethod};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
};

fn available(program: &str) -> bool {
    Command::new(program)
        .arg(match program {
            "kotlinc" => "-version",
            "go" => "version",
            _ => "--version",
        })
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

async fn exercise(extension: &str, program: &str, fixture: &str) {
    if !available(program) {
        eprintln!("Skipping {extension} JSON runtime: {program} unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let path = format!("server/controllers/JsonController.{extension}");
    let file = root.path().join(&path);
    std::fs::create_dir_all(file.parent().expect("parent")).expect("directories");
    std::fs::write(file, fixture.trim_start()).expect("controller");
    let source = HandlerSource::discover(root.path(), &path).expect("JSON controller discovery");
    assert!(
        !source.streams_method(HttpMethod::Post),
        "arrays are data, not streams"
    );
    let mut options = HandlerSupervisorOptions {
        default_timeout: Duration::from_mins(2),
        ..HandlerSupervisorOptions::default()
    };
    if extension == "ts" {
        options.runtimes.javascript = "bun".into();
    }
    let supervisor = HandlerSupervisor::new(options).expect("supervisor");
    for (route, method, expected) in [
        (
            "/json",
            HttpMethod::Get,
            serde_json::json!({"amount":42,"active":true}),
        ),
        (
            "/json",
            HttpMethod::Post,
            serde_json::json!([1, "two", null]),
        ),
        (
            "/json",
            HttpMethod::Put,
            serde_json::json!("{\"not\":\"an object\"}"),
        ),
        ("/json", HttpMethod::Delete, serde_json::Value::Null),
        (
            "/json/parsing",
            HttpMethod::Get,
            serde_json::json!({"rejected":true,"unicode":"😀"}),
        ),
        (
            "/json",
            HttpMethod::Patch,
            serde_json::json!({"explicit":true}),
        ),
    ] {
        let request = HandlerRequest::route(format!("json_{extension}_{method:?}"), route, method);
        let response = supervisor
            .invoke(&source, &request, &HandlerCancellation::default())
            .await
            .expect("JSON controller invocation");
        assert_json_response(response, extension, method, &expected);
    }
}

fn assert_json_response(
    response: HandlerResponse,
    extension: &str,
    method: HttpMethod,
    expected: &serde_json::Value,
) {
    assert!(
        response.error.is_none(),
        "{extension} {method:?}: {:?}",
        response.error
    );
    assert_eq!(
        response.status,
        if method == HttpMethod::Patch {
            201
        } else {
            200
        },
        "{extension} {method:?}"
    );
    assert!(
        response.headers["content-type"]
            .iter()
            .any(|value| value.starts_with("application/json"))
    );
    if method == HttpMethod::Patch {
        assert_eq!(response.headers["x-mode"], ["explicit"]);
    }
    let body = response.body.expect("JSON body").data;
    let actual: serde_json::Value =
        serde_json::from_str(&body).expect("exactly one JSON serialization");
    assert_eq!(actual, *expected, "{extension} {method:?}");
}

struct Fixtures;

impl Fixtures {
    const JAVASCRIPT: &str = r#"
@RequestRoute("/json")
@Public
@NeedsAll()
class JsonController {
  @GetRoute
  getJson() { return {amount:42,active:true}; }
  @PostRoute
  postJson() { return [1,"two",null]; }
  @PutRoute
  putJson() { return '{"not":"an object"}'; }
  @DeleteRoute
  deleteJson() { return null; }
  @PatchRoute
  patchJson() { return new Response('{"explicit":true}', {status:201,headers:{"content-type":"application/json","x-mode":"explicit"}}); }
  @GetRoute("/parsing")
  getParsing() {
    const malformed = ['', '{', '[1,]', '{"x" 1}', 'true false', '01', '"\\q"', '"raw\nline"'];
    let rejected = true;
    for (const text of malformed) { try { JSON.parse(text); rejected = false; } catch {} }
    return {rejected, unicode:JSON.parse('"\\ud83d\\ude00"')};
  }
}
"#;

    const PYTHON: &str = r#"
from http import HTTPStatus

@request_route("/json")
@public
@needs_all()
class JsonController:
    @get_route
    def get_json(self): return {"amount":42,"active":True}
    @post_route
    def post_json(self): return [1,"two",None]
    @put_route
    def put_json(self): return '{"not":"an object"}'
    @delete_route
    def delete_json(self): return None
    @patch_route
    def patch_json(self): return (HTTPStatus.CREATED, {"content-type":"application/json","x-mode":"explicit"}, '{"explicit":true}')
    @get_route("/parsing")
    def get_parsing(self):
        import json
        malformed = ['', '{', '[1,]', '{"x" 1}', 'true false', '01', '"\\q"', '"raw\nline"']
        rejected = True
        for text in malformed:
            try:
                json.loads(text)
                rejected = False
            except ValueError: pass
        return {"rejected":rejected,"unicode":json.loads('"\\ud83d\\ude00"')}
"#;

    const PHP: &str = r#"<?php
#[RequestRoute("/json")]
#[Public]
#[NeedsAll()]
class JsonController {
    #[GetRoute]
    public function getJson() { return ["amount"=>42,"active"=>true]; }
    #[PostRoute]
    public function postJson() { return [1,"two",null]; }
    #[PutRoute]
    public function putJson() { return '{"not":"an object"}'; }
    #[DeleteRoute]
    public function deleteJson() { return null; }
    #[PatchRoute]
    public function patchJson() {
        http_response_code(201);
        header('Content-Type: application/json');
        header('x-mode: explicit');
        echo '{"explicit":true}';
    }
    #[GetRoute("/parsing")]
    public function getParsing() {
        $malformed = ['', '{', '[1,]', '{"x" 1}', 'true false', '01', '"\\q"', "\"raw\nline\""];
        $rejected = true;
        foreach ($malformed as $text) { try { json_decode($text, true, 64, JSON_THROW_ON_ERROR); $rejected = false; } catch (JsonException $error) {} }
        return ["rejected"=>$rejected,"unicode"=>json_decode('"\\ud83d\\ude00"', true, 64, JSON_THROW_ON_ERROR)];
    }
}
"#;

    const JAVA: &str = r#"
@RequestRoute("/json")
@Public
@NeedsAll()
class JsonController {
    @GetRoute
    Object getJson() { return java.util.Map.of("amount",42,"active",true); }
    @PostRoute
    Object postJson() { return new Object[]{1,"two",null}; }
    @PutRoute
    String putJson() { return "{\"not\":\"an object\"}"; }
    @DeleteRoute
    Object deleteJson() { return null; }
    @PatchRoute
    void patchJson(com.sun.net.httpserver.HttpExchange exchange) throws java.io.IOException {
        byte[] body = "{\"explicit\":true}".getBytes(java.nio.charset.StandardCharsets.UTF_8);
        exchange.getResponseHeaders().add("content-type", "application/json");
        exchange.getResponseHeaders().add("x-mode", "explicit");
        exchange.sendResponseHeaders(201, body.length);
        exchange.getResponseBody().write(body);
    }
    @GetRoute("/parsing")
    Object getParsing() {
        String[] malformed = {"", "{", "[1,]", "{\"x\" 1}", "true false", "01", "\"\\q\"", "\"raw\nline\""};
        boolean rejected = true;
        for (String text : malformed) { try { Json.parse(text); rejected = false; } catch (IllegalArgumentException error) {} }
        try { Json.stringify(Double.NaN); rejected = false; } catch (IllegalArgumentException error) {}
        try { Json.parse("[".repeat(66) + "0" + "]".repeat(66)); rejected = false; } catch (IllegalArgumentException error) {}
        rejected &= Json.stringify(Json.parse("9223372036854775807")).equals("9223372036854775807");
        return java.util.Map.of("rejected",rejected,"unicode",Json.parse("\"\\ud83d\\ude00\""));
    }
}
"#;

    const KOTLIN: &str = r#"
@RequestRoute("/json")
@Public
@NeedsAll()
class JsonController {
    @GetRoute
    fun getJson(): Any = mapOf("amount" to 42,"active" to true)
    @PostRoute
    fun postJson(): Any = arrayOf<Any?>(1,"two",null)
    @PutRoute
    fun putJson(): String = "{\"not\":\"an object\"}"
    @DeleteRoute
    fun deleteJson(): Any? = null
    @PatchRoute
    fun patchJson(exchange: com.sun.net.httpserver.HttpExchange) {
        val body = "{\"explicit\":true}".toByteArray()
        exchange.responseHeaders.add("content-type", "application/json")
        exchange.responseHeaders.add("x-mode", "explicit")
        exchange.sendResponseHeaders(201, body.size.toLong())
        exchange.responseBody.write(body)
    }
    @GetRoute("/parsing")
    fun getParsing(): Any {
        val malformed = listOf("", "{", "[1,]", "{\"x\" 1}", "true false", "01", "\"\\q\"", "\"raw\nline\"")
        var rejected = true
        for (text in malformed) { try { Json.parse(text); rejected = false } catch (error: IllegalArgumentException) {} }
        try { Json.stringify(Double.NaN); rejected = false } catch (error: IllegalArgumentException) {}
        try { Json.parse("[".repeat(66) + "0" + "]".repeat(66)); rejected = false } catch (error: IllegalArgumentException) {}
        rejected = rejected && Json.stringify(Json.parse("9223372036854775807")) == "9223372036854775807"
        return mapOf("rejected" to rejected,"unicode" to Json.parse("\"\\ud83d\\ude00\""))
    }
}
"#;

    const CSHARP: &str = r#"
[RequestRoute("/json")]
[Public]
[NeedsAll()]
class JsonController {
    [GetRoute]
    public object GetJson() { return new {amount=42,active=true}; }
    [PostRoute]
    public object PostJson() { return new object[]{1,"two",null}; }
    [PutRoute]
    public string PutJson() { return "{\"not\":\"an object\"}"; }
    [DeleteRoute]
    public object DeleteJson() { return null; }
    [PatchRoute]
    public HttpResponseMessage PatchJson() {
        var response = new HttpResponseMessage(System.Net.HttpStatusCode.Created) { Content = new StringContent("{\"explicit\":true}", System.Text.Encoding.UTF8, "application/json") };
        response.Headers.Add("x-mode", "explicit");
        return response;
    }
    [GetRoute("/parsing")]
    public object GetParsing() {
        var malformed = new[]{"", "{", "[1,]", "{\"x\" 1}", "true false", "01", "\"\\q\"", "\"raw\nline\""};
        bool rejected = true;
        foreach (var text in malformed) { try { Json.Parse(text); rejected = false; } catch (System.Text.Json.JsonException) {} }
        return new {rejected,unicode=Json.Parse("\"\\ud83d\\ude00\"")};
    }
}
"#;

    const RUST: &str = r#"
#[RequestRoute("/json")]
#[Public]
#[NeedsAll()]
struct JsonController {}
impl JsonController {
    #[GetRoute]
    fn get_json(&self) -> Json { Json::map([("amount",Json::from(42)),("active",Json::from(true))]) }
    #[PostRoute]
    fn post_json(&self) -> Vec<Json> { vec![Json::from(1),Json::from("two"),Json::Null] }
    #[PutRoute]
    fn put_json(&self) -> &'static str { "{\"not\":\"an object\"}" }
    #[DeleteRoute]
    fn delete_json(&self) -> Option<i32> { None }
    #[PatchRoute]
    fn patch_json(&self) -> http::Response<Vec<u8>> {
        http::Response::builder()
            .status(201)
            .header("content-type", "application/json")
            .header("x-mode", "explicit")
            .body(b"{\"explicit\":true}".to_vec())
            .unwrap_or_default()
    }
    #[GetRoute("/parsing")]
    fn get_parsing(&self) -> Json {
        let malformed = ["", "{", "[1,]", "{\"x\" 1}", "true false", "01", "\"\\q\"", "\"raw\nline\""];
        let nested = "[".repeat(66) + "0" + &"]".repeat(66);
        let rejected = malformed.iter().all(|text| Json::parse(text).is_err())
            && Json::parse(&nested).is_err() && Json::from(f64::NAN).stringify().is_err()
            && Json::from(u64::MAX).stringify().expect("integer") == "18446744073709551615"
            && Json::parse("18446744073709551615").expect("integer").stringify().expect("integer") == "18446744073709551615";
        Json::map([("rejected",Json::from(rejected)),
            ("unicode",Json::parse("\"\\ud83d\\ude00\"").expect("valid unicode"))])
    }
}
"#;

    const GO: &str = r#"
import "encoding/json"
import "io"
import "net/http"
import "strings"
// @RequestRoute("/json")
// @Public
// @NeedsAll()
type JsonController struct{}

// @GetRoute
func (controller *JsonController) GetJson() any {
	return map[string]any{"amount": 42, "active": true}
}

// @PostRoute
func (controller *JsonController) PostJson() any {
	return []any{1, "two", nil}
}

// @PutRoute
func (controller *JsonController) PutJson() string {
	return `{"not":"an object"}`
}

// @DeleteRoute
func (controller *JsonController) DeleteJson() any {
	return nil
}

// @PatchRoute
func (controller *JsonController) PatchJson() *http.Response {
	header := http.Header{"Content-Type": {"application/json"}, "X-Mode": {"explicit"}}
	body := io.NopCloser(strings.NewReader(`{"explicit":true}`))
	return &http.Response{StatusCode: http.StatusCreated, Header: header, Body: body}
}

// @GetRoute("/parsing")
func (controller *JsonController) GetParsing() (any, error) {
	malformed := []string{"", "{", "[1,]", "{\"x\" 1}", "true false", "01", "\"\\q\"", "\"raw\nline\""}
	rejected := true
	for _, text := range malformed {
		var value any
		if json.Unmarshal([]byte(text), &value) == nil {
			rejected = false
		}
	}
	var unicode any
	err := json.Unmarshal([]byte("\"\\ud83d\\ude00\""), &unicode)
	return map[string]any{"rejected": rejected, "unicode": unicode}, err
}
"#;
}

#[tokio::test]
async fn javascript_values() {
    exercise("js", "node", Fixtures::JAVASCRIPT).await;
}
#[tokio::test]
async fn typescript_values() {
    exercise("ts", "bun", Fixtures::JAVASCRIPT).await;
}
#[tokio::test]
async fn python_values() {
    exercise("py", "python3", Fixtures::PYTHON).await;
}
#[tokio::test]
async fn php_values() {
    exercise("php", "php-cgi", Fixtures::PHP).await;
}
#[tokio::test]
async fn java_values() {
    exercise("java", "java", Fixtures::JAVA).await;
}
#[tokio::test]
async fn kotlin_values() {
    exercise("kt", "kotlinc", Fixtures::KOTLIN).await;
}
#[tokio::test]
async fn csharp_values() {
    exercise("cs", "dotnet", Fixtures::CSHARP).await;
}
#[tokio::test]
async fn rust_values() {
    exercise("rs", "rustc", Fixtures::RUST).await;
}
#[tokio::test]
async fn go_values() {
    exercise("go", "go", Fixtures::GO).await;
}
