#![allow(clippy::expect_used)]

use crate::support::json_http::JsonHttp;
use std::path::{Path, PathBuf};

struct LuaSchemaModels;

impl LuaSchemaModels {
    const CONFIGURATION: &'static str = "application-configuration/access-from-every-yon-language";
    const PROFILES: &'static str = "yon/reading-settings-and-choosing-configuration";
    const BINDING: &'static str = "annotations/configuration-binding-yon";
    const PAGE_SIZE: &'static str = "\"size\": 20";
    const PAGE_SIZE_ABOVE_MAX: &'static str = "\"size\": 500";
    const DESCRIBED_MODEL: &'static str = "-- @Checked\n-- @Description(\"An order line the shop accepts\")\n---@class OrderInput\nOrderInput = {\n  -- @Required\n  ---@type string\n  name = nil,\n}\n";
    const DESCRIBED_CONTROLLER: &'static str = "-- @RequestRoute(\"/described\")\n-- @Public\n---@class DescribedController\nDescribedController = {}\n\n---@return table\n-- @GetRoute\nfunction DescribedController:get_described()\n  return { description = Behaviors.records.OrderInput.description }\nend\n";

    fn lua_available() -> bool {
        let found = crate::support::program_probe::ProgramProbe::answers("lua", "-v");
        if !crate::support::toolchain::Toolchain::checked("lua", found) {
            eprintln!("skipping: lua is unavailable");
        }
        found
    }

    fn section(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../website/content/docs")
            .join(name)
            .join("server")
    }

    fn copy_lua_tab(section: &str, root: &Path) {
        Self::copy_folder(&Self::section(section), &root.join("server"));
    }

    fn copy_folder(source: &Path, target: &Path) {
        std::fs::create_dir_all(target).expect("folder");
        for entry in std::fs::read_dir(source).expect("example folder") {
            let path = entry.expect("example").path();
            let name = path.file_name().expect("file name");
            if path.is_dir() {
                Self::copy_folder(&path, &target.join(name));
            } else if path
                .extension()
                .is_some_and(|found| found == "lua" || found == "json")
            {
                std::fs::copy(&path, target.join(name)).expect("copy example");
            }
        }
    }

    fn write(root: &Path, relative: &str, text: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("folder")).expect("folder");
        std::fs::write(path, text).expect("source");
    }

    fn answer(response: &str) -> serde_json::Value {
        let (head, body) = response
            .split_once("\r\n\r\n")
            .unwrap_or_else(|| panic!("{response}"));
        assert!(head.starts_with("HTTP/1.1 200 OK"), "{response}");
        serde_json::from_str(body).unwrap_or_else(|_| panic!("JSON answer: {response}"))
    }

    fn status(response: &str) -> &str {
        response.lines().next().unwrap_or_default()
    }
}

#[tokio::test]
async fn lua_settings_and_setting_models_bind_the_application_snapshot() {
    if !LuaSchemaModels::lua_available() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    LuaSchemaModels::copy_lua_tab(LuaSchemaModels::CONFIGURATION, root.path());
    let answers = JsonHttp::get_all(root.path(), &["/configuration"]).await;
    assert_eq!(
        LuaSchemaModels::answer(&answers[0]),
        serde_json::json!({"pageSize": 20, "port": 0})
    );
    let application = root.path().join("server/application.json");
    let text = std::fs::read_to_string(&application).expect("application");
    assert!(text.contains(LuaSchemaModels::PAGE_SIZE), "{text}");
    std::fs::write(
        &application,
        text.replace(
            LuaSchemaModels::PAGE_SIZE,
            LuaSchemaModels::PAGE_SIZE_ABOVE_MAX,
        ),
    )
    .expect("page size above Max");
    let refused = JsonHttp::get_all(root.path(), &["/configuration"]).await;
    assert_eq!(
        LuaSchemaModels::status(&refused[0]),
        "HTTP/1.1 500 Internal Server Error",
        "{}",
        refused[0]
    );
}

#[tokio::test]
async fn a_lua_setting_model_reads_the_active_profile_for_the_setup() {
    if !LuaSchemaModels::lua_available() {
        return;
    }
    for section in [LuaSchemaModels::PROFILES, LuaSchemaModels::BINDING] {
        let root = tempfile::tempdir().expect("project");
        LuaSchemaModels::copy_lua_tab(section, root.path());
        let answers = JsonHttp::get_all(root.path(), &["/settings"]).await;
        let answer = LuaSchemaModels::answer(&answers[0]);
        assert_eq!(answer["profile"], "development", "{section}: {answer}");
        assert!(answer["pageSize"].is_i64(), "{section}: {answer}");
    }
}

#[tokio::test]
async fn a_lua_model_description_reaches_the_behavior_records() {
    if !LuaSchemaModels::lua_available() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    LuaSchemaModels::write(
        root.path(),
        "server/models/OrderInput.lua",
        LuaSchemaModels::DESCRIBED_MODEL,
    );
    LuaSchemaModels::write(
        root.path(),
        "server/controllers/DescribedController.lua",
        LuaSchemaModels::DESCRIBED_CONTROLLER,
    );
    let answers = JsonHttp::get_all(root.path(), &["/described"]).await;
    assert_eq!(
        LuaSchemaModels::answer(&answers[0]),
        serde_json::json!({"description": "An order line the shop accepts"})
    );
}
