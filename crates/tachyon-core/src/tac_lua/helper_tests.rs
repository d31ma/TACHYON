#![allow(clippy::expect_used)]

use super::{LuaRuntimeBuild, LuaWebServices};
use std::path::{Path, PathBuf};

struct HelperPage;

impl HelperPage {
    const FILE_FETCH: &str = "import { readFile } from 'node:fs/promises'\nglobalThis.fetch = async (url) => new Response(await readFile(url))\n;(0, eval)(await readFile('./tac-web.js', 'utf8'))\n";

    const HELPERS: [(&str, &str); 2] = [
        (
            "client/helpers/LevelHelper.lua",
            "-- @Helper\n---@class LevelHelper\nLevelHelper = {\n  -- @Needs\n  ---@type ScaleHelper\n  scale = nil,\n  ---@type integer\n  calls = 0,\n}\n\n---@param level integer\n---@return integer\nfunction LevelHelper:raised(level)\n  self.calls = self.calls + 1\n  return self.scale:times(level) + self.calls\nend\n",
        ),
        (
            "client/helpers/math/ScaleHelper.lua",
            "-- @Helper\n---@class ScaleHelper\nScaleHelper = {}\n\n---@param level integer\n---@return integer\nfunction ScaleHelper:times(level)\n  return level * 10\nend\n",
        ),
    ];

    const SERVICE: &str = "-- @Service\n---@class TallyService\nTallyService = {\n  -- @Needs\n  ---@type LevelHelper\n  levels = nil,\n  ---@type integer\n  level = 0,\n}\n\n---@param level integer\n---@return integer\nfunction TallyService:raise(level)\n  self.level = self.levels:raised(level)\n  return self.level\nend\n";

    const DRIVER: &str = "const { default: TallyService } = await import('./TallyService.js')\n\
         const first = new TallyService({}), second = new TallyService({})\n\
         const raised = [first.raise(4), first.raise(4), second.raise(4)]\n\
         console.log(JSON.stringify({ raised, level: first.level, exposed: 'levels' in first }))\n\
         process.exit(0)\n";

    const CYCLE: [(&str, &str); 2] = [
        (
            "client/helpers/LeftHelper.lua",
            "-- @Helper\n---@class LeftHelper\nLeftHelper = {\n  -- @Needs\n  ---@type RightHelper\n  right = nil,\n}\n",
        ),
        (
            "client/helpers/RightHelper.lua",
            "-- @Helper\n---@class RightHelper\nRightHelper = {\n  -- @Needs\n  ---@type LeftHelper\n  left = nil,\n}\n",
        ),
    ];

    const CYCLE_SERVICE: &str = "-- @Service\n---@class LoopService\nLoopService = {\n  -- @Needs\n  ---@type LeftHelper\n  left = nil,\n}\n\n---@return string\nfunction LoopService:name()\n  return \"loop\"\nend\n";

    const CYCLE_DRIVER: &str = "const { default: LoopService } = await import('./LoopService.js')\n\
         let failure = ''\n\
         try { new LoopService({}).name() } catch (error) { failure = String(error.message ?? error) }\n\
         console.log(JSON.stringify({ failure }))\n\
         process.exit(0)\n";

    const STORE_FILES: [(&str, &str); 4] = [
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
            "-- @Store(ProductEntity, {\"fields\":[\"id\",\"status\"],\"defaultSize\":20,\"maxSize\":100,\"setup\":\"CatalogSetup\"})\n---@class ProductStore\nProductStore = {}\n\n---@param status string\n---@return integer\nfunction ProductStore:count_by_status(status)\nend\n",
        ),
        (
            "client/helpers/ShelfHelper.lua",
            "-- @Helper\n---@class ShelfHelper\nShelfHelper = {\n  -- @Needs\n  ---@type ProductStore\n  products = nil,\n}\n\n---@return integer\nfunction ShelfHelper:active()\n  return self.products:count_by_status(\"active\")\nend\n",
        ),
    ];

    const SHELF_SERVICE: &str = "-- @Service\n---@class ShelfService\nShelfService = {\n  -- @Needs\n  ---@type ShelfHelper\n  shelf = nil,\n}\n\n---@return nil\n-- @OnStart\nfunction ShelfService:start()\n  self.behaviors = Behaviors.create(CatalogSetup, { partition = \"catalog\", signal = BehaviorSignal.new() })\nend\n\n---@param id string\n---@param status string\n---@return table\nfunction ShelfService:add(id, status)\n  return self.behaviors:repository(ProductEntity):save({ id = id, status = status })\nend\n\n---@return integer\nfunction ShelfService:active()\n  return self.shelf:active()\nend\n";

    const SHELF_DRIVER: &str = "console.info = () => {}\n\
         globalThis.indexedDB = { databases: async () => [], open: () => ({}) }\n\
         ;(0, eval)(await readFile('./tac-storage.js', 'utf8'))\n\
         const { default: ShelfService } = await import('./ShelfService.js')\n\
         const shelf = new ShelfService({})\n\
         shelf.add('p1', 'active'); shelf.add('p2', 'retired'); shelf.add('p3', 'active')\n\
         console.log(JSON.stringify({ active: shelf.active() }))\n\
         process.exit(0)\n";

    const HOOKED: &str = "-- @Helper\n---@class ScaleHelper\nScaleHelper = {}\n\n---@return nil\n-- @OnStart\nfunction ScaleHelper:start()\nend\n";

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

    fn program(root: &Path, instance: &str, class: &str, source: &str) -> String {
        let path = Self::write(root, &format!("client/services/{class}.lua"), source);
        LuaWebServices::program(root, &[(String::from(instance), String::from(class), path)])
            .unwrap_or_else(|failure| panic!("{failure}"))
    }

    async fn publish(root: &Path, instance: &str, class: &str, source: &str) {
        let path = Self::write(root, &format!("client/services/{class}.lua"), source);
        let chosen = [(
            String::from(instance),
            String::from(class),
            crate::tac_web::WebLanguage::Lua,
            path,
        )];
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
fn a_lua_service_and_its_helpers_get_their_helper_tables() {
    let root = tempfile::tempdir().expect("project");
    for (relative, text) in HelperPage::HELPERS {
        HelperPage::write(root.path(), relative, text);
    }
    let program = HelperPage::program(root.path(), "tally", "TallyService", HelperPage::SERVICE);
    for part in [
        "TallyService.__tachyonHelpers = { [\"levels\"] = \"LevelHelper\" }\n",
        "LevelHelper.__tachyonHelpers = { [\"scale\"] = \"ScaleHelper\" }\n",
        "function ScaleHelper:times(level)",
    ] {
        assert!(program.contains(part), "{part:?}:\n{program}");
    }
    assert!(
        !program.contains("ScaleHelper.__tachyonHelpers"),
        "{program}"
    );
    let registered = program
        .split("TacServices.register(")
        .last()
        .expect("registration");
    assert!(registered.contains("\"level\""), "{registered}");
    assert!(!registered.contains("\"levels\""), "{registered}");
}

#[tokio::test]
async fn a_lua_service_uses_the_helpers_it_needs_on_the_page() {
    if let Some(reason) = HelperPage::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    for (relative, text) in HelperPage::HELPERS {
        HelperPage::write(root.path(), relative, text);
    }
    HelperPage::publish(root.path(), "tally", "TallyService", HelperPage::SERVICE).await;
    let report = HelperPage::run(root.path(), HelperPage::DRIVER);
    assert_eq!(
        report["raised"],
        serde_json::json!([41, 42, 41]),
        "{report}"
    );
    assert_eq!(report["level"], 42, "{report}");
    assert_eq!(report["exposed"], false, "{report}");
}

#[tokio::test]
async fn a_lua_helper_that_needs_itself_stops_the_service_with_a_clear_message() {
    if let Some(reason) = HelperPage::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    for (relative, text) in HelperPage::CYCLE {
        HelperPage::write(root.path(), relative, text);
    }
    HelperPage::publish(
        root.path(),
        "loop",
        "LoopService",
        HelperPage::CYCLE_SERVICE,
    )
    .await;
    let report = HelperPage::run(root.path(), HelperPage::CYCLE_DRIVER);
    let failure = report["failure"].as_str().unwrap_or_default();
    assert!(
        failure.contains("The Helper LeftHelper needs itself through its -- @Needs fields"),
        "{report}"
    );
}

#[tokio::test]
async fn a_lua_helper_queries_the_store_it_needs_on_the_page() {
    if let Some(reason) = HelperPage::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    for (relative, text) in HelperPage::STORE_FILES {
        HelperPage::write(root.path(), relative, text);
    }
    HelperPage::write(
        root.path(),
        "tac-storage.js",
        include_str!("../template/tac-storage.js"),
    );
    HelperPage::publish(
        root.path(),
        "shelf",
        "ShelfService",
        HelperPage::SHELF_SERVICE,
    )
    .await;
    let report = HelperPage::run(root.path(), HelperPage::SHELF_DRIVER);
    assert_eq!(report["active"], 2, "{report}");
}

#[test]
fn a_lua_helper_with_a_lifecycle_hook_stops_the_build() {
    let root = tempfile::tempdir().expect("project");
    HelperPage::write(
        root.path(),
        "client/helpers/math/ScaleHelper.lua",
        HelperPage::HOOKED,
    );
    let service = HelperPage::SERVICE.replace("LevelHelper", "ScaleHelper");
    let path = HelperPage::write(root.path(), "client/services/TallyService.lua", &service);
    let failure = LuaWebServices::program(
        root.path(),
        &[(String::from("tally"), String::from("TallyService"), path)],
    )
    .expect_err("a Helper hook the page never runs");
    let diagnostic = &failure.diagnostics()[0];
    assert_eq!(diagnostic.code.number(), 1605, "{failure}");
    assert!(
        diagnostic
            .message
            .starts_with("The Lua Helper ScaleHelper has an OnStart, OnStop or OnEvent method."),
        "{failure}"
    );
    let start = HelperPage::HOOKED.find("-- @OnStart").expect("hook");
    let span = &diagnostic.spans[0];
    assert_eq!(
        span.file, "client/helpers/math/ScaleHelper.lua",
        "{failure}"
    );
    assert_eq!(
        usize::try_from(span.start).expect("start"),
        start,
        "{failure}"
    );
}
