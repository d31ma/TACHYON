#![allow(clippy::expect_used)]

use super::{LuaRuntimeBuild, LuaWebServices};
use std::path::{Path, PathBuf};

struct Page;

impl Page {
    const FILE_FETCH: &str = "import { readFile } from 'node:fs/promises'\nglobalThis.fetch = async (url) => new Response(await readFile(url))\n;(0, eval)(await readFile('./tac-web.js', 'utf8'))\n";

    const PRICE_SETUP: &str = "-- @Setup\n---@class PriceSetup\nPriceSetup = {}\n\n---@param execution table\n---@return table\nfunction PriceSetup.configure(execution)\n  local entries = {}\n  local cache = {\n    get = function(_, key) return entries[key] or { hit = false } end,\n    set = function(_, key, value, call) entries[key] = { hit = true, value = value, expiresAt = call.expires_at } end,\n    delete = function(_, key) entries[key] = nil end,\n  }\n  return { caches = { prices = cache }, authorize = function() return true end }\nend\n";

    const PRICES: &str = "-- @Service\n-- @CacheSettings({\"name\":\"prices\",\"ttlMs\":60000,\"maxBytes\":4096,\"maxEntries\":16})\n---@class PriceService\nPriceService = {\n  ---@type integer\n  reads = 0,\n  ---@type integer\n  attempts = 0,\n}\n\n---@return nil\n-- @OnStart\nfunction PriceService:start()\n  self.behaviors = Behaviors.create(PriceSetup, { partition = \"prices\", signal = BehaviorSignal.new() })\nend\n\n---@param sku string\n---@return integer\n-- @Cached\nfunction PriceService:price(sku)\n  self.reads = self.reads + 1\n  return #sku * 10\nend\n\n---@return integer\n-- @Retry({\"maxRetries\":3,\"delayMs\":5,\"multiplier\":1,\"maxDelayMs\":5,\"jitterMs\":0})\nfunction PriceService:steady()\n  self.attempts = self.attempts + 1\n  if self.attempts < 3 then\n    error(\"not yet\")\n  end\n  return self.attempts\nend\n";

    const PRICE_DRIVER: &str = "const { default: PriceService } = await import('./PriceService.js')\n\
         const prices = new PriceService({})\n\
         const first = prices.price('anvil'), second = prices.price('anvil')\n\
         const retried = prices.steady()\n\
         console.log(JSON.stringify({ first, second, reads: prices.reads, retried }))\n\
         process.exit(0)\n";

    const STORAGE_FILES: [(&str, &str); 2] = [
        (
            "client/setups/BrowserSetup.lua",
            "-- @Setup\n---@class BrowserSetup\nBrowserSetup = {}\n\n---@param execution table\n---@return table\nfunction BrowserSetup.configure(execution)\n  return {}\nend\n",
        ),
        (
            "client/entities/NoteEntity.lua",
            "-- @Entity({\"adapter\":\"indexeddb\"})\n-- @Table(\"notes\")\n---@class NoteEntity\nNoteEntity = {\n  -- @Id\n  ---@type string\n  id = nil,\n}\n",
        ),
    ];

    const NOTES: &str = "-- @Service\n-- @CacheSettings({\"name\":\"notes\",\"ttlMs\":60000,\"maxBytes\":4096,\"maxEntries\":8})\n---@class NotesService\nNotesService = {\n  ---@type integer\n  reads = 0,\n}\n\n---@return nil\n-- @OnStart\nfunction NotesService:start()\n  self.behaviors = Behaviors.create(BrowserSetup, { partition = \"notes\", signal = BehaviorSignal.new() })\nend\n\n---@param key string\n---@return table\n-- @Cached\nfunction NotesService:find(key)\n  self.reads = self.reads + 1\n  return { key = key, reads = self.reads }\nend\n\n---@param key string\n---@param reads integer\n---@return table\n-- @UpdateCache({\"method\":\"find\"})\nfunction NotesService:write(key, reads)\n  return { key = key, reads = reads }\nend\n\n---@param key string\n---@return nil\n-- @ClearCache({\"method\":\"find\"})\nfunction NotesService:forget(key)\nend\n\n---@param id string\n---@param text string\n---@return table\nfunction NotesService:save(id, text)\n  return self.behaviors:repository(NoteEntity):save({ id = id, text = text })\nend\n\n---@param id string\n---@return nil\n-- @AllOrNothing({\"adapter\":\"indexeddb\"})\nfunction NotesService:save_then_refuse(id)\n  self:save(id, \"draft\")\n  BehaviorError.raise(\"validation\", \"the note was refused\")\nend\n\n---@return table[]\nfunction NotesService:list()\n  return self.behaviors:repository(NoteEntity):find({})\nend\n";

    const NOTES_DRIVER: &str = "console.info = () => {}\n\
         globalThis.indexedDB = { databases: async () => [], open: () => ({}) }\n\
         ;(0, eval)(await readFile('./tac-storage.js', 'utf8'))\n\
         const { default: NotesService } = await import('./NotesService.js')\n\
         const notes = new NotesService({})\n\
         const first = notes.find('a')\n\
         const second = notes.find('a')\n\
         notes.forget('a')\n\
         const third = notes.find('a')\n\
         notes.write('a', 99)\n\
         const written = notes.find('a')\n\
         notes.save('n1', 'hello')\n\
         let refused = ''\n\
         try { notes.saveThenRefuse('n2') } catch (error) { refused = error.message }\n\
         const listed = notes.list()\n\
         console.log(JSON.stringify({ first, second, third, written, refused, listed }))\n\
         process.exit(0)\n";

    fn node_available() -> bool {
        let found = std::process::Command::new("node")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success());
        assert!(
            found || std::env::var_os("CI").is_none(),
            "CI must provide node"
        );
        found
    }

    fn skip_reason() -> Option<String> {
        if !Self::node_available() {
            return Some(String::from("node is unavailable"));
        }
        LuaRuntimeBuild::unavailable_reason()
    }

    fn write(root: &Path, relative: &str, text: &str) -> PathBuf {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(&path, text).expect("source");
        path
    }

    async fn publish(root: &Path, services: &[(&str, &str, &str)]) {
        let mut chosen = Vec::new();
        for (instance, class, source) in services {
            let path = Self::write(root, &format!("client/services/{class}.lua"), source);
            chosen.push((
                String::from(*instance),
                String::from(*class),
                crate::tac_web::WebLanguage::Lua,
                path,
            ));
        }
        let jobs = crate::tac_web::BuildJobs::new(1);
        let published =
            crate::tac_web::build_all(root, &chosen, &jobs, crate::BuildProfile::Release)
                .await
                .expect("build");
        for (name, bytes) in published.files {
            let path = root.join(name);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
            std::fs::write(path, bytes).expect("publish");
        }
    }

    fn run(root: &Path, driver: &str) -> serde_json::Value {
        Self::write(root, "tac-web.js", include_str!("../template/tac-web.js"));
        std::fs::write(
            root.join("drive.mjs"),
            format!("{}{driver}", Self::FILE_FETCH),
        )
        .expect("driver");
        let output = std::process::Command::new("node")
            .arg("drive.mjs")
            .current_dir(root)
            .output()
            .expect("node");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "{stdout}{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let last = stdout.lines().last().unwrap_or_default();
        serde_json::from_str(last).expect("report")
    }
}

#[test]
fn a_lua_service_with_behaviors_carries_the_behavior_runtime_once() {
    let root = tempfile::tempdir().expect("project");
    Page::write(
        root.path(),
        "client/setups/PriceSetup.lua",
        Page::PRICE_SETUP,
    );
    let service = Page::write(
        root.path(),
        "client/services/PriceService.lua",
        Page::PRICES,
    );
    let program = LuaWebServices::program(
        root.path(),
        &[(
            String::from("prices"),
            String::from("PriceService"),
            service,
        )],
    )
    .unwrap_or_else(|failure| panic!("{failure}"));
    assert_eq!(program.matches("function Behaviors.register(").count(), 1);
    assert!(program.contains("\"tac\""), "{program}");
}

#[tokio::test]
async fn a_lua_service_caches_and_retries_on_the_page() {
    if let Some(reason) = Page::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    Page::write(
        root.path(),
        "client/setups/PriceSetup.lua",
        Page::PRICE_SETUP,
    );
    Page::publish(root.path(), &[("prices", "PriceService", Page::PRICES)]).await;
    let report = Page::run(root.path(), Page::PRICE_DRIVER);
    assert_eq!(report["first"], 50, "{report}");
    assert_eq!(report["second"], 50, "{report}");
    assert_eq!(report["reads"], 1, "{report}");
    assert_eq!(report["retried"], 3, "{report}");
}

#[tokio::test]
async fn a_lua_service_reads_the_builtin_cache_and_store_on_the_page() {
    if let Some(reason) = Page::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    for (relative, text) in Page::STORAGE_FILES {
        Page::write(root.path(), relative, text);
    }
    Page::write(
        root.path(),
        "tac-storage.js",
        include_str!("../template/tac-storage.js"),
    );
    Page::publish(root.path(), &[("notes", "NotesService", Page::NOTES)]).await;
    let report = Page::run(root.path(), Page::NOTES_DRIVER);
    assert_eq!(report["first"], serde_json::json!({"key": "a", "reads": 1}));
    assert_eq!(report["second"], report["first"]);
    assert_eq!(report["third"], serde_json::json!({"key": "a", "reads": 2}));
    assert_eq!(
        report["written"],
        serde_json::json!({"key": "a", "reads": 99})
    );
    assert!(
        report["refused"]
            .as_str()
            .is_some_and(|message| message.contains("the note was refused")),
        "{report}"
    );
    assert_eq!(
        report["listed"],
        serde_json::json!([{"id": "n1", "text": "hello"}])
    );
}

#[test]
fn a_lua_service_keeps_browser_state_fields_on_the_page() {
    let root = tempfile::tempdir().expect("project");
    let source = "-- @Service\n---@class DraftService\nDraftService = {\n  -- @LocalState\n  ---@type string\n  draft_text = \"\",\n  -- @SessionState\n  ---@type integer\n  open_tab = 0,\n}\n\n---@param text string\n---@return nil\nfunction DraftService:write(text)\n  self.draft_text = text\nend\n";
    let service = Page::write(root.path(), "client/services/DraftService.lua", source);
    let stored = crate::behavior::StoredState::fields(&service, source)
        .unwrap_or_else(|failure| panic!("{failure}"));
    assert_eq!(
        stored.into_iter().collect::<Vec<_>>(),
        [
            (String::from("draftText"), String::from("local")),
            (String::from("openTab"), String::from("session")),
        ]
    );
    let program = LuaWebServices::program(
        root.path(),
        &[(String::from("draft"), String::from("DraftService"), service)],
    )
    .unwrap_or_else(|failure| panic!("{failure}"));
    let registration = program
        .lines()
        .find(|line| line.starts_with("TacServices.register([[draft]]"))
        .expect("registration");
    assert!(
        registration.contains("\"writable\":[\"draftText\",\"openTab\"]"),
        "{registration}"
    );
}

#[tokio::test]
async fn a_lua_service_checks_its_argument_on_the_page() {
    if let Some(reason) = Page::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    Page::write(
        root.path(),
        "client/models/Signup.lua",
        "-- @Checked\n---@class Signup\nSignup = {\n  -- @Required\n  -- @NotEmpty\n  -- @Length({\"min\":1,\"max\":8})\n  ---@type string\n  name = nil,\n  -- @Min(13)\n  -- @Max(120)\n  ---@type integer\n  age = nil,\n}\n",
    );
    let service = "-- @Service\n---@class SignupService\nSignupService = {}\n\n---@return nil\n-- @OnStart\nfunction SignupService:start()\n  self.behaviors = Behaviors.create(SignupSetup, { partition = \"signup\", signal = BehaviorSignal.new() })\nend\n\n---@param signup Signup\n---@return string\nfunction SignupService:register(\n  -- @Check(Signup)\n  signup\n)\n  return signup.name\nend\n";
    Page::write(
        root.path(),
        "client/setups/SignupSetup.lua",
        "-- @Setup\n---@class SignupSetup\nSignupSetup = {}\n\n---@param execution table\n---@return table\nfunction SignupSetup.configure(execution)\n  return {}\nend\n",
    );
    Page::publish(root.path(), &[("signup", "SignupService", service)]).await;
    let report = Page::run(
        root.path(),
        "const { default: SignupService } = await import('./SignupService.js')\n\
         const signup = new SignupService({})\n\
         const accepted = signup.register({ name: 'Ada', age: 36 })\n\
         const refused = []\n\
         for (const value of [{ age: 36 }, { name: '', age: 36 }, { name: 'Ada', age: 9 }, { name: 'Ada', age: 121 }, { name: 'Adalovelace', age: 36 }]) {\n\
         \x20 try { signup.register(value); refused.push('accepted') } catch (error) { refused.push(error.message) }\n\
         }\n\
         console.log(JSON.stringify({ accepted, refused }))\n\
         process.exit(0)\n",
    );
    assert_eq!(report["accepted"], "Ada", "{report}");
    for refused in report["refused"].as_array().expect("refused") {
        assert!(
            refused
                .as_str()
                .is_some_and(|message| message.contains("validation failed")),
            "{report}"
        );
    }
}

struct SettingsPage;

impl SettingsPage {
    const FILES: [(&str, &str); 3] = [
        (
            "client/models/CatalogSettings.lua",
            "-- @Settings({\"prefix\":\"TAC_APP_\"})\n---@class CatalogSettings\nCatalogSettings = {\n  -- @Min(1)\n  -- @Max(100)\n  ---@type integer\n  size = 20,\n}\n",
        ),
        (
            "client/models/Endpoint.lua",
            "---@class Endpoint\nEndpoint = {\n  -- @Setting({\"path\":\"tac.app.size\",\"type\":\"integer\",\"default\":20,\"min\":1,\"max\":100})\n  ---@type integer\n  size = nil,\n}\n",
        ),
        (
            "client/setups/CatalogSetup.lua",
            "-- @Setup\n-- @WhenProfile(\"test\")\n-- @WhenSetting({\"name\":\"TAC_CACHE_ENABLED\",\"havingValue\":\"true\",\"matchIfMissing\":false})\n---@class CatalogSetup\nCatalogSetup = {}\n\n---@param execution table\n---@return table\nfunction CatalogSetup.configure(execution)\n  return {}\nend\n",
        ),
    ];

    const SERVICE: &str = "-- @Service\n-- @Description(\"Reads the catalog settings\")\n-- @CacheSettings({\"name\":\"catalog\",\"ttlMs\":60000,\"maxBytes\":4096,\"maxEntries\":8})\n---@class CatalogService\nCatalogService = {\n  ---@type integer\n  reads = 0,\n}\n\n---@param profiles string[]\n---@param enabled string\n---@return string\nfunction CatalogService:open(profiles, enabled)\n  local options = { partition = \"catalog\", signal = BehaviorSignal.new(), profiles = profiles, properties = { tac = { cache = { enabled = enabled } } } }\n  local ok, failure = pcall(Behaviors.create, CatalogSetup, options)\n  if ok then\n    self.behaviors = failure\n    return \"open\"\n  end\n  return failure.behavior_code\nend\n\n---@return table\nfunction CatalogService:settings()\n  return Behaviors.properties(CatalogSettings, { tac = { app = { size = 25 } } })\nend\n\n---@return table\nfunction CatalogService:endpoint()\n  return Behaviors.values(Endpoint, { tac = { app = { size = 30 } } })\nend\n\n---@param sku string\n---@param note string\n---@return integer\n-- @Cached\nfunction CatalogService:price(\n  -- @CacheKey\n  sku,\n  note\n)\n  self.reads = self.reads + 1\n  return #sku\nend\n\n---@param again boolean\n---@return string\n-- @MaxAtOnce({\"limit\":1,\"policy\":\"reject\"})\nfunction CatalogService:once(again)\n  if again then\n    local ok, failure = pcall(self.once, self, false)\n    return ok and \"entered twice\" or failure.behavior_code\n  end\n  return \"inside\"\nend\n";

    const DRIVER: &str = "console.info = () => {}\n\
         globalThis.indexedDB = { databases: async () => [], open: () => ({}) }\n\
         ;(0, eval)(await readFile('./tac-storage.js', 'utf8'))\n\
         const { default: CatalogService } = await import('./CatalogService.js')\n\
         const catalog = new CatalogService({})\n\
         const inactive = [catalog.open([], 'true'), catalog.open(['test'], 'false')]\n\
         const opened = catalog.open(['test'], 'true')\n\
         const settings = catalog.settings()\n\
         const endpoint = catalog.endpoint()\n\
         catalog.price('anvil', 'first'); catalog.price('anvil', 'second')\n\
         const reads = catalog.reads\n\
         const once = catalog.once(true)\n\
         console.log(JSON.stringify({ inactive, opened, settings, endpoint, reads, once }))\n\
         process.exit(0)\n";
}

#[tokio::test]
async fn a_lua_service_reads_settings_profiles_and_limits_on_the_page() {
    if let Some(reason) = Page::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    for (relative, text) in SettingsPage::FILES {
        Page::write(root.path(), relative, text);
    }
    Page::write(
        root.path(),
        "tac-storage.js",
        include_str!("../template/tac-storage.js"),
    );
    Page::publish(
        root.path(),
        &[("catalog", "CatalogService", SettingsPage::SERVICE)],
    )
    .await;
    let report = Page::run(root.path(), SettingsPage::DRIVER);
    assert_eq!(
        report["inactive"],
        serde_json::json!(["inactive-configuration", "inactive-configuration"]),
        "{report}"
    );
    assert_eq!(report["opened"], "open", "{report}");
    assert_eq!(
        report["settings"],
        serde_json::json!({"size": 25}),
        "{report}"
    );
    assert_eq!(
        report["endpoint"],
        serde_json::json!({"size": 30}),
        "{report}"
    );
    assert_eq!(report["reads"], 1, "{report}");
    assert_eq!(report["once"], "capacity", "{report}");
}

struct UnitPage;

impl UnitPage {
    const FILES: [(&str, &str); 3] = [
        (
            "client/tests/UnitTestSetup.lua",
            "-- @TestSetup\n-- @WhenSetting({\"name\":\"TAC_CACHE_ENABLED\",\"havingValue\":\"true\",\"matchIfMissing\":false})\n---@class UnitTestSetup\nUnitTestSetup = {}\n\n---@param execution table\n---@return table\nfunction UnitTestSetup.configure(execution)\n  local entries = {}\n  local cache = {\n    get = function(_, key) return entries[key] or { hit = false } end,\n    set = function(_, key, value, call) entries[key] = { hit = true, value = value, expiresAt = call.expires_at } end,\n    delete = function(_, key) entries[key] = nil end,\n  }\n  return { caches = { prices = cache }, authorize = function() return true end }\nend\n",
        ),
        (
            "client/services/PriceService.lua",
            "-- @Service\n-- @CacheSettings({\"name\":\"prices\",\"ttlMs\":60000,\"maxBytes\":4096,\"maxEntries\":16})\n---@class PriceService\nPriceService = {\n  ---@type integer\n  reads = 0,\n  ---@type string\n  label = \"real\",\n}\n\n---@param sku string\n---@return integer\n-- @Cached\nfunction PriceService:price(sku)\n  self.reads = self.reads + 1\n  return #sku * 10\nend\n",
        ),
        (
            "client/tests/PriceServiceTest.lua",
            "-- @Test(UnitTestSetup)\n-- @TestSettings({\"tac.cache.enabled\":\"true\"})\n-- @FreshStart\n---@class PriceServiceTest\nPriceServiceTest = {\n  -- @Fake\n  ---@type string\n  label = nil,\n}\n\nfunction PriceServiceTest:test_a_second_price_comes_from_the_cache()\n  local prices = self:construct(PriceService)\n  prices:price(\"anvil\")\n  Assert.equals(50, prices:price(\"anvil\"))\n  Assert.equals(1, prices.reads)\nend\n\nfunction PriceServiceTest:test_each_case_starts_with_an_empty_cache()\n  local prices = self:construct(PriceService)\n  prices:price(\"anvil\")\n  Assert.equals(1, prices.reads)\nend\n\nfunction PriceServiceTest:test_a_fake_replaces_the_field()\n  self.label = \"fake\"\n  Assert.equals(\"fake\", self:construct(PriceService).label)\nend\n",
        ),
    ];
}

#[test]
fn a_lua_test_unit_gets_a_behavior_context_settings_and_fakes() {
    if crate::lua_source::LuaProgram::new().is_none() {
        eprintln!("skipping: lua is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    for (relative, text) in UnitPage::FILES {
        Page::write(root.path(), relative, text);
    }
    let report =
        crate::client_tests::run(root.path(), Path::new("client/tests/PriceServiceTest.lua"))
            .unwrap_or_else(|failure| panic!("{failure}"));
    assert_eq!(report.unit, "PriceServiceTest");
    for case in &report.cases {
        assert!(case.passed, "{}: {:?}", case.name, case.error);
    }
    assert_eq!(report.cases.len(), 3);
}

struct StorePage;

impl StorePage {
    const FILES: [(&str, &str); 3] = [
        (
            "client/setups/CatalogSetup.lua",
            "-- @Setup\n---@class CatalogSetup\nCatalogSetup = {}\n\n---@param execution table\n---@return table\nfunction CatalogSetup.configure(execution)\n  return {}\nend\n",
        ),
        (
            "client/entities/ProductEntity.lua",
            "-- @Entity({\"adapter\":\"indexeddb\"})\n-- @Table(\"products\")\n---@class ProductEntity\nProductEntity = {\n  -- @Id\n  ---@type string\n  id = nil,\n}\n",
        ),
        (
            "client/stores/ProductStore.lua",
            "-- @Store(ProductEntity, {\"fields\":[\"id\",\"status\"],\"defaultSize\":20,\"maxSize\":100,\"setup\":\"CatalogSetup\"})\n---@class ProductStore\nProductStore = {}\n\n---@param status string\n---@return table[]\nfunction ProductStore:find_by_status(status)\nend\n\n---@param status string\n---@return integer\nfunction ProductStore:count_by_status(status)\nend\n\n---@param status string\n---@param page table\n---@return table[]\n-- @Query({\"operation\":\"find\",\"where\":[{\"field\":\"status\",\"operator\":\"ne\",\"argument\":0}],\"orderBy\":[{\"field\":\"id\",\"direction\":\"desc\"}]})\nfunction ProductStore:excluding(\n  status,\n  -- @PageDefaults\n  page\n)\nend\n",
        ),
    ];

    const SERVICE: &str = "-- @Service\n---@class ProductService\nProductService = {\n  -- @Needs\n  ---@type ProductStore\n  products = nil,\n}\n\n---@return nil\n-- @OnStart\nfunction ProductService:start()\n  self.behaviors = Behaviors.create(CatalogSetup, { partition = \"catalog\", signal = BehaviorSignal.new() })\nend\n\n---@param id string\n---@param status string\n---@return table\nfunction ProductService:add(id, status)\n  return self.behaviors:repository(ProductEntity):save({ id = id, status = status })\nend\n\n---@return table\nfunction ProductService:report()\n  return { active = self.products:find_by_status(\"active\"), count = self.products:count_by_status(\"active\"), others = self.products:excluding(\"active\", { size = 5 }) }\nend\n";

    const DRIVER: &str = "console.info = () => {}\n\
         globalThis.indexedDB = { databases: async () => [], open: () => ({}) }\n\
         ;(0, eval)(await readFile('./tac-storage.js', 'utf8'))\n\
         const { default: ProductService } = await import('./ProductService.js')\n\
         const products = new ProductService({})\n\
         products.add('p1', 'active'); products.add('p2', 'retired'); products.add('p3', 'active'); products.add('p4', 'draft')\n\
         console.log(JSON.stringify(products.report()))\n\
         process.exit(0)\n";
}

#[tokio::test]
async fn a_lua_service_queries_its_store_on_the_page() {
    if let Some(reason) = Page::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    for (relative, text) in StorePage::FILES {
        Page::write(root.path(), relative, text);
    }
    Page::write(
        root.path(),
        "tac-storage.js",
        include_str!("../template/tac-storage.js"),
    );
    Page::publish(
        root.path(),
        &[("products", "ProductService", StorePage::SERVICE)],
    )
    .await;
    let report = Page::run(root.path(), StorePage::DRIVER);
    assert_eq!(
        report["active"],
        serde_json::json!([{"id": "p1", "status": "active"}, {"id": "p3", "status": "active"}]),
        "{report}"
    );
    assert_eq!(report["count"], 2, "{report}");
    assert_eq!(
        report["others"],
        serde_json::json!([{"id": "p4", "status": "draft"}, {"id": "p2", "status": "retired"}]),
        "{report}"
    );
}

struct HttpPage;

impl HttpPage {
    const FILES: [(&str, &str); 2] = [
        (
            "client/setups/CatalogSetup.lua",
            "-- @Setup\n---@class CatalogSetup\nCatalogSetup = {}\n\n---@param execution table\n---@return table\nfunction CatalogSetup.configure(execution)\n  local echo = {\n    exchange = function(_, request, call)\n      return { method = request.method, path = request.path, query = request.query, headers = request.headers, body = request.body }\n    end,\n  }\n  return { http = { catalog = echo } }\nend\n",
        ),
        (
            "client/remotes/CatalogClient.lua",
            "-- @HTTPClient({\"path\":\"/api\"})\n---@class CatalogClient\nCatalogClient = {}\n\n---@param id string\n---@param view string\n---@param tenant string\n---@return table\n-- @GetRequest(\"/products/{id}\")\nfunction CatalogClient:find(\n  -- @PathVariable(\"id\")\n  id,\n  -- @RequestParam(\"view\")\n  view,\n  -- @RequestHeader(\"X-Tenant\")\n  tenant\n)\nend\n\n---@param product table\n---@return table\n-- @PostRequest(\"/products\")\nfunction CatalogClient:save(\n  -- @RequestBody\n  product\n)\nend\n\n---@param id integer\n---@return table\n-- @DeleteRequest(\"/products/{id}\")\nfunction CatalogClient:remove(\n  -- @PathVariable(\"id\")\n  id\n)\nend\n",
        ),
    ];

    const SERVICE: &str = "-- @Service\n---@class CatalogService\nCatalogService = {\n  -- @Needs\n  ---@type CatalogClient\n  catalog_client = nil,\n}\n\n---@return table\nfunction CatalogService:report()\n  return { found = self.catalog_client:find(\"anvil\", \"full\", \"acme\"), saved = self.catalog_client:save({ sku = \"anvil\" }), removed = self.catalog_client:remove(7) }\nend\n";

    const DRIVER: &str = "const { default: CatalogService } = await import('./CatalogService.js')\n\
         const catalog = new CatalogService({})\n\
         console.log(JSON.stringify(catalog.report()))\n\
         process.exit(0)\n";
}

#[tokio::test]
async fn a_lua_http_client_sends_each_request_through_its_setup_adapter_on_the_page() {
    if let Some(reason) = Page::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    for (relative, text) in HttpPage::FILES {
        Page::write(root.path(), relative, text);
    }
    Page::publish(
        root.path(),
        &[("catalog", "CatalogService", HttpPage::SERVICE)],
    )
    .await;
    let report = Page::run(root.path(), HttpPage::DRIVER);
    assert_eq!(
        report["found"],
        serde_json::json!({"method": "GET", "path": "/api/products/anvil", "query": {"view": "full"}, "headers": {"X-Tenant": "acme"}}),
        "{report}"
    );
    assert_eq!(
        report["saved"],
        serde_json::json!({"method": "POST", "path": "/api/products", "query": {}, "headers": {}, "body": {"sku": "anvil"}}),
        "{report}"
    );
    assert_eq!(report["removed"]["path"], "/api/products/7", "{report}");
}
