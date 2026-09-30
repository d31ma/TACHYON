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
