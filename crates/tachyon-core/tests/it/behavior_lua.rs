#![allow(clippy::expect_used)]

use std::process::{Command, Stdio};
use tachyon_contracts::{HandlerRequest, HttpMethod};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
};

struct LuaBehaviors;

impl LuaBehaviors {
    const CACHING: [(&str, &str); 6] = [
        (
            "server/models/CacheReport.lua",
            "---@class CacheReport\n---@field first any\n---@field repeat any\n---@field written any\n---@field refreshed any\n---@field retried integer\nCacheReport = {}\n",
        ),
        (
            "server/helpers/MemoryCacheHelper.lua",
            "-- @Helper\n---@class MemoryCacheHelper\nMemoryCacheHelper = {\n  ---@type table<string, table>\n  entries = {},\n}\n\n---@param key string\n---@param execution table\n---@return table\nfunction MemoryCacheHelper:get(key, execution)\n  return self.entries[key] or { hit = false }\nend\n\n---@param key string\n---@param value any\n---@param execution table\n---@return nil\nfunction MemoryCacheHelper:set(key, value, execution)\n  self.entries[key] = { hit = true, value = value, expiresAt = execution.expires_at }\nend\n\n---@param key string\n---@param execution table\n---@return nil\nfunction MemoryCacheHelper:delete(key, execution)\n  self.entries[key] = nil\nend\n\n---@param call table\n---@param execution table\n---@return boolean\nfunction MemoryCacheHelper.allow(call, execution)\n  return true\nend\n",
        ),
        (
            "server/setups/CatalogSetup.lua",
            "-- @Setup\n---@class CatalogSetup\nCatalogSetup = {}\n\n---@param execution table\n---@return table\nfunction CatalogSetup.configure(execution)\n  return { caches = { catalog = MemoryCacheHelper }, authorize = MemoryCacheHelper.allow }\nend\n",
        ),
        (
            "server/services/CatalogService.lua",
            "-- @Service\n-- @CacheSettings({\"name\":\"catalog\",\"ttlMs\":30000,\"maxBytes\":65536,\"maxEntries\":256})\n---@class CatalogService\nCatalogService = {\n  ---@type integer\n  reads = 0,\n  ---@type integer\n  attempts = 0,\n}\n\n---@return nil\n-- @OnStart\nfunction CatalogService:open()\n  self.behaviors = Behaviors.create(CatalogSetup, { partition = \"public-catalog\", signal = BehaviorSignal.new() })\nend\n\n---@param sku string\n---@return table\n-- @Cached\nfunction CatalogService:find(sku)\n  self.reads = self.reads + 1\n  return { sku = sku, reads = self.reads }\nend\n\n---@param sku string\n---@param reads integer\n---@return table\n-- @UpdateCache({\"method\":\"find\"})\nfunction CatalogService:write(sku, reads)\n  return { sku = sku, reads = reads }\nend\n\n---@param sku string\n---@return table\n-- @ClearCache({\"method\":\"find\"})\nfunction CatalogService:invalidate(sku)\n  return { sku = sku, reads = self.reads }\nend\n\n---@return integer\n-- @Retry({\"maxRetries\":3,\"delayMs\":1,\"multiplier\":2,\"maxDelayMs\":5,\"jitterMs\":0})\nfunction CatalogService:steady()\n  self.attempts = self.attempts + 1\n  if self.attempts < 3 then\n    error(\"not yet\")\n  end\n  return self.attempts\nend\n",
        ),
        (
            "server/controllers/CatalogController.lua",
            "-- @RequestRoute(\"/catalog\")\n-- @Public\n---@class CatalogController\nCatalogController = {\n  -- @Needs\n  ---@type CatalogService\n  catalog_service = nil,\n}\n\n---@return CacheReport\n-- @GetRoute\nfunction CatalogController:get_catalog()\n  local sku = \"anvil\"\n  local first = self.catalog_service:find(sku)\n  local again = self.catalog_service:find(sku)\n  self.catalog_service:write(sku, 99)\n  local written = self.catalog_service:find(sku)\n  self.catalog_service:invalidate(sku)\n  local refreshed = self.catalog_service:find(sku)\n  return { first = first, [\"repeat\"] = again, written = written, refreshed = refreshed, retried = self.catalog_service:steady() }\nend\n",
        ),
        (
            "server/tests/CatalogServiceTest.lua",
            "-- @Test(CatalogSetup, {\"partition\":\"tests\"})\n---@class CatalogServiceTest\nCatalogServiceTest = {}\n\nfunction CatalogServiceTest:test_a_second_find_comes_from_the_cache()\n  local catalog = self:construct(CatalogService)\n  catalog:find(\"anvil\")\n  Assert.equals(1, catalog:find(\"anvil\").reads)\nend\n\nfunction CatalogServiceTest:test_steady_retries_until_it_works()\n  Assert.equals(3, self:construct(CatalogService):steady())\nend\n",
        ),
    ];
}

fn lua_available() -> bool {
    Command::new("lua")
        .arg("-v")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn project() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("project");
    for (relative, text) in LuaBehaviors::CACHING {
        let path = root.path().join(relative);
        std::fs::create_dir_all(path.parent().expect("folder")).expect("folder");
        std::fs::write(path, text).expect("source");
    }
    root
}

async fn call(
    root: &std::path::Path,
    entry: &str,
    route: &str,
    method: HttpMethod,
) -> serde_json::Value {
    let source = HandlerSource::discover(root, entry).unwrap_or_else(|failure| panic!("{failure}"));
    let supervisor =
        HandlerSupervisor::new(HandlerSupervisorOptions::default()).expect("supervisor");
    let request = HandlerRequest::route("lua-behaviors", route, method);
    let response = supervisor
        .invoke(&source, &request, &HandlerCancellation::default())
        .await
        .expect("invocation");
    assert!(response.error.is_none(), "{:?}", response.error);
    let body = response.body.expect("body").data;
    serde_json::from_str(&body).unwrap_or_else(|_| panic!("JSON answer: {body}"))
}

#[tokio::test]
async fn a_lua_service_caches_updates_clears_and_retries() {
    if !lua_available() {
        eprintln!("skipping: lua is unavailable");
        return;
    }
    let root = project();
    let answer = call(
        root.path(),
        "server/controllers/CatalogController.lua",
        "/catalog",
        HttpMethod::Get,
    )
    .await;
    assert_eq!(
        answer,
        serde_json::json!({
            "first": {"sku": "anvil", "reads": 1},
            "repeat": {"sku": "anvil", "reads": 1},
            "written": {"sku": "anvil", "reads": 99},
            "refreshed": {"sku": "anvil", "reads": 2},
            "retried": 3,
        })
    );
}

#[tokio::test]
async fn a_lua_test_unit_gets_a_behavior_context_for_each_case() {
    if !lua_available() {
        eprintln!("skipping: lua is unavailable");
        return;
    }
    let root = project();
    let report = call(
        root.path(),
        "server/tests/CatalogServiceTest.lua",
        "/",
        HttpMethod::Post,
    )
    .await;
    let cases = report["cases"].as_array().expect("cases");
    assert_eq!(cases.len(), 2, "{report}");
    for case in cases {
        assert_eq!(case["passed"], true, "{report}");
    }
}

#[tokio::test]
async fn a_lua_stream_route_yields_each_value_from_a_coroutine() {
    if !lua_available() {
        eprintln!("skipping: lua is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    for (relative, text) in [
        (
            "server/models/Ticket.lua",
            "---@class Ticket\n---@field sku string\n---@field level integer\nTicket = {}\n",
        ),
        (
            "server/services/TicketService.lua",
            "-- @Service\n---@class TicketService\nTicketService = {}\n\n---@return integer[]\nfunction TicketService:levels()\n  return { 1, 2, 3 }\nend\n",
        ),
        (
            "server/controllers/EventsController.lua",
            "-- @RequestRoute(\"/events\")\n-- @Public\n---@class EventsController\nEventsController = {\n  -- @Needs\n  ---@type TicketService\n  ticket_service = nil,\n}\n\n---@return nil\n-- @StreamRoute\nfunction EventsController:stream_events()\n  self:_tickets_for(self.ticket_service:levels())\nend\n\n---@param levels integer[]\n---@return nil\nfunction EventsController:_tickets_for(levels)\n  for _, level in ipairs(levels) do\n    coroutine.yield(setmetatable({ sku = \"anvil\", level = level }, Ticket))\n  end\nend\n",
        ),
    ] {
        let path = root.path().join(relative);
        std::fs::create_dir_all(path.parent().expect("folder")).expect("folder");
        std::fs::write(path, text).expect("source");
    }
    let source = HandlerSource::discover(root.path(), "server/controllers/EventsController.lua")
        .unwrap_or_else(|failure| panic!("{failure}"));
    assert!(source.streams_method(HttpMethod::Get));
    let mut request = HandlerRequest::route("events", "/events", HttpMethod::Get);
    request.arguments = Some(Vec::new());
    let supervisor =
        HandlerSupervisor::new(HandlerSupervisorOptions::default()).expect("supervisor");
    let (sender, mut receiver) = tokio::sync::mpsc::channel(8);
    supervisor
        .invoke_streaming(&source, &request, sender)
        .await
        .expect("stream");
    let mut events = Vec::new();
    while let Some(event) = receiver.recv().await {
        events.push(event.data);
    }
    assert_eq!(
        events,
        [
            "{\"sku\":\"anvil\",\"level\":1}",
            "{\"sku\":\"anvil\",\"level\":2}",
            "{\"sku\":\"anvil\",\"level\":3}"
        ]
    );
}

struct LuaClients;

impl LuaClients {
    const CATALOG: [(&str, &str); 5] = [
        (
            "server/helpers/EchoHttpHelper.lua",
            "-- @Helper\n---@class EchoHttpHelper\nEchoHttpHelper = {}\n\n---@param request table\n---@param execution table\n---@return table\nfunction EchoHttpHelper:exchange(request, execution)\n  return { method = request.method, path = request.path, query = request.query, headers = request.headers, body = request.body, partition = execution.partition }\nend\n",
        ),
        (
            "server/setups/CatalogSetup.lua",
            "-- @Setup\n---@class CatalogSetup\nCatalogSetup = {}\n\n---@param execution table\n---@return table\nfunction CatalogSetup.configure(execution)\n  return { http = { catalog = EchoHttpHelper } }\nend\n",
        ),
        (
            "server/remotes/CatalogClient.lua",
            "-- @HTTPClient({\"path\":\"/api\"})\n---@class CatalogClient\nCatalogClient = {}\n\n---@param id string\n---@param view string\n---@param tenant string\n---@return table\n-- @GetRequest(\"/products/{id}\")\nfunction CatalogClient:find(\n  -- @PathVariable(\"id\")\n  id,\n  -- @RequestParam(\"view\")\n  view,\n  -- @RequestHeader(\"X-Tenant\")\n  tenant\n)\nend\n\n---@param product table\n---@return table\n-- @PostRequest(\"/products\")\nfunction CatalogClient:save(\n  -- @RequestBody\n  product\n)\nend\n\n---@param id integer\n---@return table\n-- @DeleteRequest(\"/products/{id}\")\nfunction CatalogClient:remove(\n  -- @PathVariable(\"id\")\n  id\n)\nend\n",
        ),
        (
            "server/controllers/CatalogController.lua",
            "-- @RequestRoute(\"/catalog\")\n-- @Public\n---@class CatalogController\nCatalogController = {\n  -- @Needs\n  ---@type CatalogClient\n  catalog_client = nil,\n}\n\n---@return table\n-- @GetRoute\nfunction CatalogController:get_catalog()\n  return { found = self.catalog_client:find(\"anvil\", \"full\", \"acme\"), saved = self.catalog_client:save({ sku = \"anvil\" }), removed = self.catalog_client:remove(7) }\nend\n",
        ),
        (
            "server/tests/CatalogClientTest.lua",
            "-- @Test(CatalogSetup, {\"partition\":\"tests\"})\n---@class CatalogClientTest\nCatalogClientTest = {}\n\nfunction CatalogClientTest:test_a_path_variable_fills_the_path()\n  Assert.equals(\"/api/products/anvil\", self:construct(CatalogClient):find(\"anvil\", \"full\", \"acme\").path)\nend\n",
        ),
    ];
}

fn write_project(files: &[(&str, &str)]) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("project");
    for (relative, text) in files {
        let path = root.path().join(relative);
        std::fs::create_dir_all(path.parent().expect("folder")).expect("folder");
        std::fs::write(path, text).expect("source");
    }
    root
}

#[tokio::test]
async fn a_lua_http_client_sends_each_request_through_its_setup_adapter() {
    if !lua_available() {
        eprintln!("skipping: lua is unavailable");
        return;
    }
    let root = write_project(&LuaClients::CATALOG);
    let answer = call(
        root.path(),
        "server/controllers/CatalogController.lua",
        "/catalog",
        HttpMethod::Get,
    )
    .await;
    assert_eq!(
        answer,
        serde_json::json!({
            "found": {
                "method": "GET",
                "path": "/api/products/anvil",
                "query": {"view": "full"},
                "headers": {"X-Tenant": "acme"},
                "partition": "catalog",
            },
            "saved": {
                "method": "POST",
                "path": "/api/products",
                "query": {},
                "headers": {},
                "body": {"sku": "anvil"},
                "partition": "catalog",
            },
            "removed": {
                "method": "DELETE",
                "path": "/api/products/7",
                "query": {},
                "headers": {},
                "partition": "catalog",
            },
        })
    );
}

#[tokio::test]
async fn a_lua_test_unit_constructs_an_http_client_with_its_setup() {
    if !lua_available() {
        eprintln!("skipping: lua is unavailable");
        return;
    }
    let root = write_project(&LuaClients::CATALOG);
    let report = call(
        root.path(),
        "server/tests/CatalogClientTest.lua",
        "/",
        HttpMethod::Post,
    )
    .await;
    let cases = report["cases"].as_array().expect("cases");
    assert_eq!(cases.len(), 1, "{report}");
    assert_eq!(cases[0]["passed"], true, "{report}");
}
