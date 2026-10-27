#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;
use std::time::Duration;
use tachyon_contracts::{
    HandlerBody, HandlerBodyEncoding, HandlerRequest, HandlerResponse, HttpMethod,
};
use tachyon_core::{
    ApplicationConfig, HandlerCancellation, HandlerSource, HandlerSupervisor,
    HandlerSupervisorOptions, WarmSettings, YonIsolationPolicy,
};

const LUA_COUNTER: &str = r#"---@class Handler
Handler = {
  ---@type integer
  count = 0,
  ---@type string
  identity = "",
}

---@param request table
---@return table
function Handler.POST(request)
  if request.body == "crash" then
    os.exit(23)
  end
  if Handler.identity == "" then
    Handler.identity = string.format("%s-%d", tostring({}), math.random(1, 1000000000))
  end
  Handler.count = Handler.count + 1
  return { identity = Handler.identity, count = Handler.count }
end
"#;

const LUA_STREAM: &str = r#"---@class Handler
Handler = {
  ---@type string
  identity = "",
}

---@param request table
---@return table
function Handler.POST(request)
  if Handler.identity == "" then
    Handler.identity = string.format("%s-%d", tostring({}), math.random(1, 1000000000))
  end
  return YonStream.of(function()
    for index = 1, 3 do
      coroutine.yield({ identity = Handler.identity, index = index })
    end
  end)
end
"#;

const PYTHON_SERVICE: (&str, &str) = (
    "server/services/ReportService.py",
    r#"@service
class ReportService:
    def summarize(self, mode, detail):
        return {"language": "python", "mode": mode, "detail": detail}
"#,
);

const LUA_CLIENT: (&str, &str) = (
    "server/remotes/ReportClient.lua",
    "-- @HTTPClient\n---@class ReportClient\nReportClient = {}\n\n---@param mode any\n---@param detail any\n---@return any\n-- @Gateway({\"class\":\"server/services/ReportService.py\"})\nfunction ReportClient:summarize(mode, detail)\nend\n",
);

const LUA_CONTROLLER: (&str, &str) = (
    "server/controllers/ReportController.lua",
    "-- @RequestRoute(\"/report\")\n-- @Public\n---@class ReportController\nReportController = {\n  -- @Needs\n  ---@type ReportClient\n  report_client = nil,\n  ---@type integer\n  served = 0,\n}\n\n---@param input any[]\n---@return table\n-- @PostRoute\nfunction ReportController:post_report(\n  -- @FromBody\n  input\n)\n  ReportController.served = ReportController.served + 1\n  local answer = self.report_client:summarize(input[1], input[2])\n  return { language = answer.language, detail = answer.detail, served = ReportController.served }\nend\n",
);

fn program_available(program: &str, flag: &str) -> bool {
    let found = crate::support::program_probe::ProgramProbe::answers(program, flag);
    let required = cfg!(unix) && std::env::var_os("CI").is_some();
    assert!(
        found || !required,
        "{program} is required for the Lua warm handler tests in CI"
    );
    if !found {
        eprintln!("skipping Lua warm handler execution: {program} is unavailable");
    }
    found
}

fn lua_available() -> bool {
    program_available("lua", "-v")
}

fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    fs::write(path, contents).expect("file");
}

fn source(root: &tempfile::TempDir, contents: &str) -> HandlerSource {
    let relative = "server/workers/warm.lua";
    write(root.path(), relative, contents);
    HandlerSource::discover(root.path(), relative).expect("validated source")
}

fn request(id: &str, mode: &str) -> HandlerRequest {
    let mut request = HandlerRequest::route(id, "/warm", HttpMethod::Post);
    request.body = Some(HandlerBody {
        encoding: HandlerBodyEncoding::Utf8,
        data: String::from(mode),
    });
    request
}

fn warm_isolation() -> YonIsolationPolicy {
    YonIsolationPolicy::Warm(WarmSettings::new(1_000, Duration::from_mins(1)))
}

fn supervisor(isolation: YonIsolationPolicy) -> HandlerSupervisor {
    HandlerSupervisor::new(HandlerSupervisorOptions {
        isolation,
        default_timeout: Duration::from_secs(10),
        ..HandlerSupervisorOptions::default()
    })
    .expect("supervisor")
}

fn body_json(response: &HandlerResponse) -> serde_json::Value {
    let body = response.body.as_ref().expect("successful body");
    serde_json::from_str(&body.data).expect("JSON handler result")
}

async fn answer(supervisor: &HandlerSupervisor, source: &HandlerSource, id: &str) -> (String, u64) {
    let response = supervisor
        .invoke(source, &request(id, ""), &HandlerCancellation::default())
        .await
        .expect("Lua response");
    let value = body_json(&response);
    let identity = value["identity"].as_str().expect("identity").to_owned();
    let count = value["count"].as_u64().expect("count");
    (identity, count)
}

async fn report(
    supervisor: &HandlerSupervisor,
    source: &HandlerSource,
    detail: &str,
) -> serde_json::Value {
    let arguments = serde_json::json!(["store", detail]);
    let mut request = HandlerRequest::route("warm_gateway", "/report", HttpMethod::Post);
    request.arguments = Some(vec![arguments.clone()]);
    request.body = Some(HandlerBody {
        encoding: HandlerBodyEncoding::Utf8,
        data: arguments.to_string(),
    });
    let response = supervisor
        .invoke(source, &request, &HandlerCancellation::default())
        .await
        .expect("gateway caller response");
    body_json(&response)
}

#[tokio::test]
async fn a_warm_lua_handler_serves_later_requests_from_the_same_process() {
    if !lua_available() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = source(&root, LUA_COUNTER);
    let supervisor = supervisor(warm_isolation());
    let (first, first_count) = answer(&supervisor, &source, "warm_first").await;
    let (second, second_count) = answer(&supervisor, &source, "warm_second").await;
    assert_eq!(first, second, "the Lua process was reused");
    assert_eq!(first_count, 1);
    assert_eq!(second_count, 2, "the Lua handler kept its module state");
}

#[tokio::test]
async fn lua_process_mode_starts_a_new_process_for_every_request() {
    if !lua_available() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = source(&root, LUA_COUNTER);
    let supervisor = supervisor(YonIsolationPolicy::Process);
    let (_, first_count) = answer(&supervisor, &source, "process_first").await;
    let (_, second_count) = answer(&supervisor, &source, "process_second").await;
    assert_eq!([first_count, second_count], [1, 1]);
}

#[tokio::test]
async fn a_lua_crash_replaces_the_warm_process() {
    if !lua_available() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = source(&root, LUA_COUNTER);
    let supervisor = supervisor(warm_isolation());
    let (before, _) = answer(&supervisor, &source, "before_crash").await;
    let crashed = supervisor
        .invoke(
            &source,
            &request("crash", "crash"),
            &HandlerCancellation::default(),
        )
        .await
        .expect_err("crash");
    assert!(crashed.to_string().contains("TY2104"), "{crashed}");
    let (after, count) = answer(&supervisor, &source, "after_crash").await;
    assert_ne!(before, after, "a new Lua process started");
    assert_eq!(count, 1, "the new Lua process started with fresh state");
}

#[tokio::test]
async fn a_warm_lua_stream_ends_with_its_end_frame_and_stays_warm() {
    if !lua_available() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let source = source(&root, LUA_STREAM);
    let supervisor = supervisor(warm_isolation());
    let mut identities = Vec::new();
    for id in ["stream_first", "stream_second"] {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(8);
        let streamed = request(id, "");
        let invocation = supervisor.invoke_streaming(&source, &streamed, sender);
        let collect = async {
            let mut events = Vec::new();
            while let Some(event) = receiver.recv().await {
                events.push(event);
            }
            events
        };
        let (result, events) = tokio::join!(invocation, collect);
        result.expect("stream finished");
        assert_eq!(events.len(), 3, "{id}");
        let first: serde_json::Value = serde_json::from_str(&events[0].data).expect("event");
        identities.push(first["identity"].as_str().expect("identity").to_owned());
    }
    assert_eq!(identities[0], identities[1]);
}

#[tokio::test]
async fn a_warm_lua_gateway_exchanges_resource_frames_for_every_request() {
    if !lua_available() || !program_available("python3", "--version") {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    write(
        root.path(),
        "server/application.json",
        r#"{"datasource":{"driver":"sqlite","url":"sqlite::memory:"}}"#,
    );
    for (relative, contents) in [PYTHON_SERVICE, LUA_CLIENT, LUA_CONTROLLER] {
        write(root.path(), relative, contents);
    }
    let source = HandlerSource::discover(root.path(), LUA_CONTROLLER.0).expect("gateway caller");
    let application = ApplicationConfig::load(root.path()).expect("application");
    let supervisor = HandlerSupervisor::new(HandlerSupervisorOptions {
        application,
        isolation: warm_isolation(),
        default_timeout: Duration::from_secs(10),
        ..HandlerSupervisorOptions::default()
    })
    .expect("supervisor");
    let first = report(&supervisor, &source, "first").await;
    let second = report(&supervisor, &source, "second").await;
    assert_eq!(first["language"], "python", "{first}");
    assert_eq!(first["detail"], "first", "{first}");
    assert_eq!(second["detail"], "second", "{second}");
    let served = [first["served"].as_u64(), second["served"].as_u64()];
    assert_eq!(served, [Some(1), Some(2)], "the warm Lua process was reused");
}
