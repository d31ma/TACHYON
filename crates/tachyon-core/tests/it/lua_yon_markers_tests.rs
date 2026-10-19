#![allow(clippy::expect_used)]

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;
use tachyon_contracts::{HandlerRequest, HttpMethod};
use tachyon_core::{
    DevServer, DevServerOptions, HandlerCancellation, HandlerSource, HandlerSupervisor,
    HandlerSupervisorOptions, ProjectDiscovery,
};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpStream;

struct LuaInjection;

impl LuaInjection {
    const FILES: [(&str, &str); 5] = [
        (
            "server/services/SharedCounterService.lua",
            r#"-- @Service
-- @Lifetime("singleton")
---@class SharedCounterService
SharedCounterService = {
  ---@type integer
  count = 0,
}

---@return integer
function SharedCounterService:next()
  self.count = self.count + 1
  return self.count
end
"#,
        ),
        (
            "server/services/FreshCounterService.lua",
            r#"-- @Service
-- @Lifetime("transient")
---@class FreshCounterService
FreshCounterService = {
  ---@type integer
  count = 0,
}

---@return integer
function FreshCounterService:next()
  self.count = self.count + 1
  return self.count
end
"#,
        ),
        (
            "server/services/StandardPricingService.lua",
            r#"-- @Service
---@class StandardPricingService
StandardPricingService = {}

---@param amount integer
---@return table
function StandardPricingService:quote(amount)
  return { provider = "standard", amount = amount }
end
"#,
        ),
        (
            "server/services/DiscountPricingService.lua",
            r#"-- @Service
---@class DiscountPricingService
DiscountPricingService = {}

---@param amount integer
---@return table
function DiscountPricingService:quote(amount)
  return { provider = "discount", amount = amount * 8 // 10 }
end
"#,
        ),
        (
            "server/controllers/InjectionController.lua",
            r#"-- @RequestRoute("/injection")
-- @Public
---@class InjectionController
InjectionController = {
  -- @Needs
  ---@type SharedCounterService
  first_shared = nil,
  -- @Needs
  ---@type SharedCounterService
  second_shared = nil,
  -- @Needs
  ---@type FreshCounterService
  first_fresh = nil,
  -- @Needs
  ---@type FreshCounterService
  second_fresh = nil,
  -- @Needs
  -- @Named("DiscountPricingService")
  ---@type StandardPricingService
  pricing = nil,
  -- @Needs
  ---@type StandardPricingService
  standard = nil,
}

---@return table
-- @GetRoute
function InjectionController:get_injection()
  self.first_shared:next()
  self.first_fresh:next()
  return {
    shared_same = rawequal(self.first_shared, self.second_shared),
    shared_count = self.second_shared:next(),
    fresh_same = rawequal(self.first_fresh, self.second_fresh),
    fresh_count = self.second_fresh:next(),
    named = self.pricing:quote(100),
    unnamed = self.standard:quote(100),
  }
end
"#,
        ),
    ];
}

struct LuaBehaviorMarkers;

impl LuaBehaviorMarkers {
    const FILES: [(&str, &str); 8] = [
        (
            "server/helpers/LedgerAdapterHelper.lua",
            r#"-- @Helper
---@class LedgerAdapterHelper
LedgerAdapterHelper = {
  ---@type string[]
  commands = {},
  ---@type integer
  sequence = 0,
  ---@type table[]
  ROWS = { { id = 1, status = "open" }, { id = 2, status = "settled" } },
}

---@param command table
---@param execution table
---@return any
function LedgerAdapterHelper:execute(command, execution)
  if command.operation == "transaction" then
    return self:_transaction(command)
  end
  local query = command.query
  self.commands[#self.commands + 1] = query.kind .. "@" .. (command.transaction or "none")
  local matching = {}
  for _, row in ipairs(self.ROWS) do
    if row.status == query.where[1].value then
      matching[#matching + 1] = row
    end
  end
  if query.kind == "count" then
    return #matching
  end
  return matching
end

---@param command table
---@return table
function LedgerAdapterHelper:_transaction(command)
  local identifier = command.transaction
  self.commands[#self.commands + 1] = identifier and (command.action .. ":" .. identifier) or command.action
  if command.action ~= "begin" then
    return { ok = true }
  end
  self.sequence = self.sequence + 1
  return { transaction = "t" .. self.sequence }
end
"#,
        ),
        (
            "server/setups/LedgerSetup.lua",
            r"-- @Setup
---@class LedgerSetup
LedgerSetup = {}

---@param execution table
---@return table
function LedgerSetup.configure(execution)
  return { entities = { ledger = LedgerAdapterHelper }, authorizeEntity = LedgerSetup.authorize }
end

---@param command table
---@param execution table
---@return boolean
function LedgerSetup.authorize(command, execution)
  return true
end
",
        ),
        (
            "server/entities/EntryEntity.lua",
            r#"-- @Entity({"adapter":"ledger"})
-- @Table("entries")
---@class EntryEntity
EntryEntity = {
  -- @Id
  ---@type integer
  id = nil,
}
"#,
        ),
        (
            "server/stores/EntryStore.lua",
            r#"-- @Store(EntryEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100,"setup":"LedgerSetup"})
---@class EntryStore
EntryStore = {}

---@param status string
---@return table[]
function EntryStore:find_by_status(status)
end

---@param status string
---@return integer
function EntryStore:count_by_status(status)
end
"#,
        ),
        (
            "server/services/LedgerService.lua",
            r#"-- @Service
-- @Description("Settles the open ledger entries")
---@class LedgerService
LedgerService = {
  -- @Needs
  ---@type EntryStore
  entry_store = nil,
}

---@return nil
-- @OnStart
function LedgerService:open()
  self.behaviors = Behaviors.create(LedgerSetup, { partition = "ledger", signal = BehaviorSignal.new() })
end

---@param status string
---@return table
-- @AllOrNothing({"adapter":"ledger"})
function LedgerService:settle(status)
  return { open = self.entry_store:find_by_status(status), total = self.entry_store:count_by_status(status) }
end

---@param status string
---@return table
-- @AllOrNothing({"adapter":"ledger"})
function LedgerService:settle_then_fail(status)
  self.entry_store:find_by_status(status)
  error({ reason = "the ledger is locked" })
end

---@param depth integer
---@return string
-- @MaxAtOnce({"limit":2,"policy":"reject"})
function LedgerService:nested(depth)
  if depth == 0 then
    return "ran"
  end
  local ok, result = pcall(self.nested, self, depth - 1)
  if ok then
    return result
  end
  return result.behavior_code
end
"#,
        ),
        (
            "server/setups/CatalogSetup.lua",
            r#"-- @Setup
-- @Settings({"prefix":"YON_CATALOG_"})
-- @WhenProfile("development")
-- @WhenSetting({"name":"YON_CATALOG_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})
---@class CatalogSetup
CatalogSetup = {
  -- @Min(1)
  -- @Max(100)
  ---@type integer
  page_size = 10,
  ---@type boolean
  cache_enabled = false,
}

---@param execution table
---@return table
function CatalogSetup.configure(execution)
  return {}
end
"#,
        ),
        (
            "server/controllers/LedgerController.lua",
            r#"-- @RequestRoute("/ledger")
-- @Public
---@class LedgerController
LedgerController = {
  -- @Needs
  ---@type LedgerService
  ledger_service = nil,
}

---@return table
-- @GetRoute
function LedgerController:get_ledger()
  local settled = self.ledger_service:settle("open")
  local ok, failure = pcall(self.ledger_service.settle_then_fail, self.ledger_service, "open")
  return {
    settled = settled,
    failed = not ok,
    reason = failure.reason,
    commands = LedgerAdapterHelper.commands,
    description = Behaviors.records.LedgerService.description,
    within_limit = self.ledger_service:nested(1),
    over_limit = self.ledger_service:nested(2),
    after_release = self.ledger_service:nested(1),
  }
end
"#,
        ),
        (
            "server/controllers/SettingsController.lua",
            r#"-- @RequestRoute("/settings")
-- @Public
---@class SettingsController
SettingsController = {}

---@param values table
---@return any
function SettingsController:_bound(values)
  local ok, result = pcall(Behaviors.properties, CatalogSetup, values)
  if ok then
    return result
  end
  return result.behavior_code
end

---@param profiles string[]
---@param values table
---@return string
function SettingsController:_selected(profiles, values)
  local ok, result = pcall(Behaviors.create, CatalogSetup, { partition = "catalog", signal = BehaviorSignal.new(), profiles = profiles, properties = values })
  if not ok then
    return result.behavior_code
  end
  result:dispose()
  return "selected"
end

---@return table
-- @GetRoute
function SettingsController:get_settings()
  local enabled = { YON_CATALOG_CACHE_ENABLED = "true" }
  return {
    bound = self:_bound({ YON_CATALOG_PAGE_SIZE = "20", YON_CATALOG_CACHE_ENABLED = "true" }),
    application = self:_bound({ yon = { catalog = { page = { size = 30 }, cache = { enabled = false } } } }),
    undeclared = self:_bound({ YON_CATALOG_PAGE_SIZE = "20", YON_CATALOG_CACHE_ENABLED = "true", YON_CATALOG_COLOR = "red" }),
    not_a_number = self:_bound({ YON_CATALOG_PAGE_SIZE = "twenty", YON_CATALOG_CACHE_ENABLED = "true" }),
    at_min = self:_bound({ YON_CATALOG_PAGE_SIZE = "1", YON_CATALOG_CACHE_ENABLED = "false" }),
    below_min = self:_bound({ YON_CATALOG_PAGE_SIZE = "0", YON_CATALOG_CACHE_ENABLED = "false" }),
    at_max = self:_bound({ YON_CATALOG_PAGE_SIZE = "100", YON_CATALOG_CACHE_ENABLED = "false" }),
    above_max = self:_bound({ YON_CATALOG_PAGE_SIZE = "101", YON_CATALOG_CACHE_ENABLED = "false" }),
    active = self:_selected({ "development" }, enabled),
    other_profile = self:_selected({ "production" }, enabled),
    no_profile = self:_selected({}, enabled),
    setting_off = self:_selected({ "development" }, { YON_CATALOG_CACHE_ENABLED = "false" }),
    setting_missing = self:_selected({ "development" }, {}),
  }
end
"#,
        ),
    ];
}

struct LuaHttpMarkers;

impl LuaHttpMarkers {
    const FILES: [(&str, &str); 11] = [
        ("client/pages/tac.html", "<test-view1 />"),
        (
            "client/components/test/view1/tac.html",
            "<main aria-label=\"Home\"><h1>Home</h1></main>",
        ),
        ("server/roles/Admin.lua", "---@class Admin\nAdmin = {}\n"),
        (
            "server/models/Identity.lua",
            "---@class Identity\n---@field user string\n---@field roles string[]\nIdentity = {}\n",
        ),
        (
            "server/controllers/AppIdentityProvider.lua",
            r#"-- @IdentityProvider
---@class AppIdentityProvider
AppIdentityProvider = {}

---@param request table
---@return Identity|nil
function AppIdentityProvider:identify(request)
  return self:_find(request.headers["authorization"])
end

---@param token string|nil
---@return Identity|nil
function AppIdentityProvider:_find(token)
  if token == "ada-token" then
    return { user = "ada", roles = { "Admin" } }
  end
  if token == "bob-token" then
    return { user = "bob", roles = {} }
  end
  return nil
end
"#,
        ),
        (
            "server/controllers/OrdersController.lua",
            r#"-- @RequestRoute("/orders")
-- @SignedIn
---@class OrdersController
OrdersController = {}

---@return string[]
-- @GetRoute
-- @Public
function OrdersController:get_orders()
  return { "listed-lamp" }
end

---@return string[]
-- @PostRoute
function OrdersController:post_order()
  return { "posted-chair" }
end

---@return string[]
-- @DeleteRoute
-- @Allow(Admin)
function OrdersController:delete_orders()
  return { "deleted-all" }
end
"#,
        ),
        (
            "server/models/Item.lua",
            "---@class Item\n---@field name string\n---@field level integer\nItem = {}\n",
        ),
        (
            "server/models/Problem.lua",
            "---@class Problem\n---@field code string\n---@field message string\nProblem = {}\n",
        ),
        (
            "server/errors/ItemNotFound.lua",
            "---@class ItemNotFound\n---@field message string\nItemNotFound = {}\n",
        ),
        (
            "server/controllers/ItemsController.lua",
            r#"-- @RequestRoute("/items")
-- @Public
---@class ItemsController
ItemsController = {}

---@param id integer
---@return Item
-- @GetRoute("/{id}")
function ItemsController:get_item(
  -- @FromPath
  id
)
  if id > 99 then
    error(setmetatable({ message = "item " .. id }, ItemNotFound))
  end
  return { name = "anvil", level = id }
end

---@param id integer
---@param item Item
---@return Item
-- @PatchRoute("/{id}")
function ItemsController:patch_item(
  -- @FromPath
  id,
  -- @FromBody
  item
)
  return { name = "patched-" .. item.name, level = id }
end

---@param id integer
---@return Item
-- @DeleteRoute("/{id}")
function ItemsController:delete_item(
  -- @FromPath
  id
)
  return { name = "deleted", level = id }
end

---@param item Item
---@return Item
-- @PostRoute
-- @ResponseStatus(201)
function ItemsController:post_item(
  -- @FromBody
  item
)
  return { name = "created-" .. item.name, level = item.level }
end
"#,
        ),
        (
            "server/controllers/ItemsErrorHandler.lua",
            r#"-- @ErrorHandler
---@class ItemsErrorHandler
ItemsErrorHandler = {}

---@param failure ItemNotFound
---@return Problem
-- @HandlesError
-- @ResponseStatus(404)
function ItemsErrorHandler:missing(failure)
  return { code = "item-not-found", message = failure.message }
end
"#,
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

fn write_project(files: &[(&str, &str)]) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("project");
    for (relative, text) in files {
        let path = root.path().join(relative);
        std::fs::create_dir_all(path.parent().expect("folder")).expect("folder");
        std::fs::write(path, text).expect("source");
    }
    root
}

async fn call(root: &Path, entry: &str, route: &str) -> serde_json::Value {
    let source = HandlerSource::discover(root, entry).unwrap_or_else(|failure| panic!("{failure}"));
    let supervisor =
        HandlerSupervisor::new(HandlerSupervisorOptions::default()).expect("supervisor");
    let request = HandlerRequest::route("lua-markers", route, HttpMethod::Get);
    let response = supervisor
        .invoke(&source, &request, &HandlerCancellation::default())
        .await
        .expect("invocation");
    assert!(response.error.is_none(), "{:?}", response.error);
    let body = response.body.expect("body").data;
    serde_json::from_str(&body).unwrap_or_else(|_| panic!("JSON answer: {body}"))
}

#[tokio::test]
async fn lua_lifetime_and_named_pick_the_instance_each_needs_field_gets() {
    if !lua_available() {
        eprintln!("skipping: lua is unavailable");
        return;
    }
    let root = write_project(&LuaInjection::FILES);
    let answer = call(
        root.path(),
        "server/controllers/InjectionController.lua",
        "/injection",
    )
    .await;
    assert_eq!(
        answer,
        serde_json::json!({
            "shared_same": true,
            "shared_count": 2,
            "fresh_same": false,
            "fresh_count": 1,
            "named": {"provider": "discount", "amount": 80},
            "unnamed": {"provider": "standard", "amount": 100},
        })
    );
}

#[tokio::test]
async fn lua_all_or_nothing_max_at_once_and_description_run_in_a_service() {
    if !lua_available() {
        eprintln!("skipping: lua is unavailable");
        return;
    }
    let root = write_project(&LuaBehaviorMarkers::FILES);
    let answer = call(
        root.path(),
        "server/controllers/LedgerController.lua",
        "/ledger",
    )
    .await;
    assert_eq!(
        answer,
        serde_json::json!({
            "settled": {"open": [{"id": 1, "status": "open"}], "total": 1},
            "failed": true,
            "reason": "the ledger is locked",
            "commands": [
                "begin", "find@t1", "count@t1", "commit:t1",
                "begin", "find@t2", "rollback:t2",
            ],
            "description": "Settles the open ledger entries",
            "within_limit": "ran",
            "over_limit": "capacity",
            "after_release": "ran",
        })
    );
}

#[tokio::test]
async fn lua_settings_bind_bounded_fields_and_select_the_setup_by_profile_and_setting() {
    if !lua_available() {
        eprintln!("skipping: lua is unavailable");
        return;
    }
    let root = write_project(&LuaBehaviorMarkers::FILES);
    let answer = call(
        root.path(),
        "server/controllers/SettingsController.lua",
        "/settings",
    )
    .await;
    let refused = "configuration-properties";
    let inactive = "inactive-configuration";
    assert_eq!(
        answer,
        serde_json::json!({
            "bound": {"page_size": 20, "cache_enabled": true},
            "application": {"page_size": 30, "cache_enabled": false},
            "undeclared": refused,
            "not_a_number": refused,
            "at_min": {"page_size": 1, "cache_enabled": false},
            "below_min": refused,
            "at_max": {"page_size": 100, "cache_enabled": false},
            "above_max": refused,
            "active": "selected",
            "other_profile": inactive,
            "no_profile": inactive,
            "setting_off": inactive,
            "setting_missing": inactive,
        })
    );
}

struct Server {
    port: u16,
    stop: tokio::sync::oneshot::Sender<()>,
    running: tokio::task::JoinHandle<Result<(), tachyon_core::Failure>>,
}

impl Server {
    async fn start(root: &Path) -> Self {
        let project =
            ProjectDiscovery::discover(root).unwrap_or_else(|failure| panic!("{failure}"));
        let server = DevServer::bind_project(
            &project,
            &DevServerOptions {
                port: 0,
                watch: false,
                ..DevServerOptions::default()
            },
        )
        .await
        .expect("bind");
        let port = server.address().port();
        let (stop, shutdown) = tokio::sync::oneshot::channel();
        let running = tokio::spawn(server.run_until(async {
            let _ = shutdown.await;
        }));
        Self {
            port,
            stop,
            running,
        }
    }

    async fn call(&self, method: &str, path: &str, headers: &str, body: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", self.port))
            .await
            .expect("connect");
        let raw = format!(
            "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(raw.as_bytes()).await.expect("write");
        let mut response = Vec::new();
        tokio::time::timeout(Duration::from_mins(5), stream.read_to_end(&mut response))
            .await
            .expect("answer within the deadline")
            .expect("read");
        String::from_utf8_lossy(&response).into_owned()
    }

    async fn stop(self) {
        self.stop.send(()).expect("stop");
        self.running.await.expect("server task").expect("shutdown");
    }
}

struct Exchange {
    method: &'static str,
    path: &'static str,
    headers: &'static str,
    body: &'static str,
    status: &'static str,
    contains: &'static str,
    absent: &'static str,
}

impl Exchange {
    const ADA: &'static str = "Authorization: ada-token\r\n";
    const BOB: &'static str = "Authorization: bob-token\r\n";
    const ITEM: &'static str = "{\"name\":\"hammer\",\"level\":3}";

    const ALL: [Self; 12] = [
        Self::new(
            "GET",
            "/items/7",
            "",
            "",
            "200 OK",
            "{\"name\":\"anvil\",\"level\":7}",
        ),
        Self::new(
            "PATCH",
            "/items/7",
            "",
            Self::ITEM,
            "200 OK",
            "{\"name\":\"patched-hammer\",\"level\":7}",
        ),
        Self::new(
            "DELETE",
            "/items/7",
            "",
            "",
            "200 OK",
            "{\"name\":\"deleted\",\"level\":7}",
        ),
        Self::new(
            "PUT",
            "/items/7",
            "",
            Self::ITEM,
            "405 Method Not Allowed",
            "",
        )
        .without("hammer"),
        Self::new(
            "POST",
            "/items",
            "",
            Self::ITEM,
            "201 Created",
            "{\"name\":\"created-hammer\",\"level\":3}",
        ),
        Self::new(
            "GET",
            "/items/500",
            "",
            "",
            "404 Not Found",
            "{\"code\":\"item-not-found\",\"message\":\"item 500\"}",
        ),
        Self::new("GET", "/orders", "", "", "200 OK", "listed-lamp"),
        Self::new("POST", "/orders", "", "", "401 Unauthorized", "").without("posted-chair"),
        Self::new("POST", "/orders", Self::BOB, "", "200 OK", "posted-chair"),
        Self::new("DELETE", "/orders", "", "", "401 Unauthorized", "").without("deleted-all"),
        Self::new("DELETE", "/orders", Self::BOB, "", "403 Forbidden", "").without("deleted-all"),
        Self::new("DELETE", "/orders", Self::ADA, "", "200 OK", "deleted-all"),
    ];

    const fn new(
        method: &'static str,
        path: &'static str,
        headers: &'static str,
        body: &'static str,
        status: &'static str,
        contains: &'static str,
    ) -> Self {
        Self {
            method,
            path,
            headers,
            body,
            status,
            contains,
            absent: "\u{0}",
        }
    }

    const fn without(self, absent: &'static str) -> Self {
        Self { absent, ..self }
    }
}

#[tokio::test]
async fn lua_routes_answer_their_verb_status_error_handler_and_access_rule() {
    if !lua_available() {
        eprintln!("skipping: lua is unavailable");
        return;
    }
    let root = write_project(&LuaHttpMarkers::FILES);
    let server = Server::start(root.path()).await;
    for exchange in &Exchange::ALL {
        let answer = server
            .call(
                exchange.method,
                exchange.path,
                exchange.headers,
                exchange.body,
            )
            .await;
        let label = format!("{} {}", exchange.method, exchange.path);
        assert!(
            answer.starts_with(&format!("HTTP/1.1 {}", exchange.status)),
            "{label}: {answer}"
        );
        assert!(answer.contains(exchange.contains), "{label}: {answer}");
        assert!(!answer.contains(exchange.absent), "{label}: {answer}");
    }
    server.stop().await;
}
