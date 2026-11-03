use super::{BuildOptions, WebCompiler};
use crate::node_driver::NodeDriver;
use serde_json::json;
use std::path::{Path, PathBuf};

struct ScriptMarkerProofs;

impl ScriptMarkerProofs {
    const FIXTURES: &'static str = "tests/fixtures/tac-script-markers";
    const WEBSITE_SECTIONS: &'static str = "../../website/content/docs/annotations";
    const VIEWS: [&'static str; 3] = [
        "pages/tac.html",
        "components/proof/panel/tac.html",
        "manifest.json",
    ];
    const SCRIPTS: [&'static str; 9] = [
        "services/SettingsService",
        "services/ShelfService",
        "services/GateService",
        "services/LedgerService",
        "services/TillService",
        "setups/ShelfSetup",
        "entities/BookEntity",
        "stores/BookStore",
        "models/LedgerEntry",
    ];
    const SECTION_FILES: [(&'static str, &'static str, &'static str); 8] = [
        (
            "caching-tac",
            "services/CatalogService",
            "services/CatalogService",
        ),
        ("caching-tac", "setups/CatalogSetup", "setups/CacheSetup"),
        (
            "retries-tac",
            "services/OrdersService",
            "services/OrdersService",
        ),
        ("retries-tac", "setups/OrdersSetup", "setups/OrdersSetup"),
        ("validation-tac", "models/OrderInput", "models/OrderInput"),
        (
            "validation-tac",
            "services/OrderService",
            "services/OrderService",
        ),
        (
            "configuration-binding-tac",
            "setups/CatalogSetup",
            "setups/CatalogSetup",
        ),
        (
            "configuration-binding-tac",
            "models/PublicValues",
            "models/PublicValues",
        ),
    ];
    const CACHING_SECTION: &'static str = "caching-tac";
    const RUNTIME: [(&'static str, &'static str); 4] = [
        ("package.json", "{\"type\":\"module\"}"),
        ("tac-storage.js", include_str!("../template/tac-storage.js")),
        (
            "application-events.js",
            concat!(
                include_str!("../template/application-events.js"),
                "\nexport default TacApplicationEvents\n"
            ),
        ),
        (
            "drive.mjs",
            include_str!("../../tests/fixtures/tac-script-markers/drive.mjs"),
        ),
    ];

    fn crate_folder() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn write(path: &Path, text: &str) {
        std::fs::create_dir_all(path.parent().unwrap_or_else(|| panic!("parent")))
            .unwrap_or_else(|failure| panic!("folder: {failure}"));
        std::fs::write(path, text).unwrap_or_else(|failure| panic!("file: {failure}"));
    }

    fn read(path: &Path) -> String {
        std::fs::read_to_string(path)
            .unwrap_or_else(|failure| panic!("{}: {failure}", path.display()))
    }

    fn section_text(section: &str, text: String) -> String {
        if section == Self::CACHING_SECTION {
            return text.replace("CatalogSetup", "CacheSetup");
        }
        text
    }

    fn project(root: &Path, extension: &str) {
        let fixtures = Self::crate_folder().join(Self::FIXTURES).join(extension);
        let client = root.join("client");
        for view in Self::VIEWS {
            Self::write(
                &client.join(view),
                &Self::read(&fixtures.join("client").join(view)),
            );
        }
        for script in Self::SCRIPTS {
            let file = format!("{script}.{extension}");
            Self::write(
                &client.join(&file),
                &Self::read(&fixtures.join("client").join(&file)),
            );
        }
        let sections = Self::crate_folder().join(Self::WEBSITE_SECTIONS);
        for (section, source, target) in Self::SECTION_FILES {
            let read = sections
                .join(section)
                .join("client")
                .join(format!("{source}.{extension}"));
            let text = Self::section_text(section, Self::read(&read));
            Self::write(&client.join(format!("{target}.{extension}")), &text);
        }
    }

    async fn report(extension: &str) -> serde_json::Value {
        let root = tempfile::tempdir().unwrap_or_else(|failure| panic!("project: {failure}"));
        Self::project(root.path(), extension);
        let result = WebCompiler::build_async(root.path(), &BuildOptions::default())
            .await
            .unwrap_or_else(|failure| panic!("the {extension} services build: {failure}"));
        let output = result.output_directory();
        for (name, text) in Self::RUNTIME {
            Self::write(&output.join(name), text);
        }
        let mut command = NodeDriver::async_command();
        command.arg("drive.mjs").current_dir(output);
        let captured = NodeDriver::run_within(&mut command, std::time::Duration::from_mins(1))
            .await
            .unwrap_or_else(|failure| panic!("supervised emitted module execution: {failure}"));
        let printed = String::from_utf8_lossy(&captured.stdout);
        assert!(
            captured.status.success(),
            "{printed}{}",
            String::from_utf8_lossy(&captured.stderr)
        );
        let last = printed.lines().last().unwrap_or_default();
        serde_json::from_str(last).unwrap_or_else(|failure| panic!("{failure}: {printed}"))
    }

    fn assert_cache_and_resilience(report: &serde_json::Value) {
        assert_eq!(
            report["cache"],
            json!({
                "first": {"sku": "anvil", "reads": 1},
                "again": {"sku": "anvil", "reads": 1},
                "written": {"sku": "anvil", "reads": 99},
                "updated": {"sku": "anvil", "reads": 99},
                "cleared": {"sku": "anvil", "reads": 2},
            }),
            "{report}"
        );
        let resilience = &report["resilience"];
        assert_eq!(
            resilience["submitted"],
            json!({"id": "o1", "attempts": 3}),
            "{report}"
        );
        assert_eq!(
            resilience["rejected"][0],
            json!({"value": "first"}),
            "{report}"
        );
        assert_eq!(resilience["rejected"][1]["error"], "capacity", "{report}");
        assert_eq!(resilience["blocked"], json!(["a", "b"]), "{report}");
        assert_eq!(
            resilience["steps"],
            json!(["start a", "end a", "start b", "end b"]),
            "{report}"
        );
        assert_eq!(
            resilience["exhausted"]["message"], "try 3 failed",
            "{report}"
        );
        assert_eq!(resilience["tries"], 3, "{report}");
    }

    fn assert_validation_and_settings(report: &serde_json::Value) {
        let validation = &report["validation"];
        assert_eq!(
            validation["accepted"],
            json!({"value": {"name": "Tea", "level": 2}}),
            "{report}"
        );
        for (rule, path) in [
            ("NotEmpty", "name.notBlank"),
            ("Length", "name.size"),
            ("Min", "level.min"),
            ("Max", "level.max"),
        ] {
            assert_eq!(validation[rule]["error"], "validation", "{rule}: {report}");
            assert_eq!(validation[rule]["path"], path, "{rule}: {report}");
        }
        let settings = &report["settings"];
        assert_eq!(
            settings["bound"],
            json!({"value": {"pageSize": 35, "cacheEnabled": true, "title": "Shop"}}),
            "{report}"
        );
        assert_eq!(settings["bounds"][0]["error"], "configuration-properties");
        assert_eq!(settings["bounds"][1]["error"], "configuration-properties");
        assert_eq!(settings["bounds"][2]["value"]["pageSize"], 100, "{report}");
        assert_eq!(
            settings["values"],
            json!([{"value": {"title": "Shop"}}, {"value": {"title": "Catalog"}}]),
            "{report}"
        );
        assert_eq!(settings["gates"][0], json!({"value": "open"}), "{report}");
        assert_eq!(
            [1, 2, 3].map(|closed| settings["gates"][closed]["error"].clone()),
            [
                json!("inactive-configuration"),
                json!("inactive-configuration"),
                json!("inactive-configuration")
            ],
            "{report}"
        );
    }

    fn assert_store(report: &serde_json::Value) {
        let store = &report["store"];
        let lent = json!([{"code": "b6", "status": "lent"}]);
        assert_eq!(
            store["found"],
            json!({"active": lent, "count": 2, "others": lent, "full": lent}),
            "{report}"
        );
        assert_eq!(store["oversized"]["error"], "invalid", "{report}");
        assert_eq!(
            store["pages"][0]["value"],
            json!({"page": 0, "size": 20, "offset": 0, "sort": []}),
            "{report}"
        );
        assert_eq!(
            store["pages"][1]["value"],
            json!({"page": 2, "size": 20, "offset": 40, "sort": ["code:desc"]}),
            "{report}"
        );
        assert_eq!(store["pages"][2]["error"], "invalid", "{report}");
        assert_eq!(store["pages"][3]["error"], "invalid", "{report}");
        let book = json!({"entity": "BookEntity", "table": "books", "idField": "code"});
        let query = |kind: &str, operator: &str, order: serde_json::Value| {
            let mut command = book.clone();
            command["operation"] = json!("query");
            command["query"] = json!({"kind": kind, "where": [{"field": "status", "operator": operator, "value": "active"}], "orderBy": order});
            command
        };
        let paged = |mut command: serde_json::Value, size: u64| {
            command["query"]["page"] = json!({"page": 0, "size": size, "offset": 0, "sort": []});
            command
        };
        let ascending = json!([{"field": "code", "direction": "asc"}]);
        let descending = json!([{"field": "code", "direction": "desc"}]);
        let saved = |code: &str, status: &str, transaction: &str| {
            let mut command = book.clone();
            command["operation"] = json!("save");
            command["value"] = json!({"code": code, "status": status});
            command["transaction"] = json!(transaction);
            command
        };
        assert_eq!(
            store["commands"],
            json!([
                paged(query("find", "eq", ascending), 2),
                query("count", "eq", json!([])),
                paged(query("find", "ne", descending.clone()), 2),
                paged(query("find", "ne", descending), 3),
                {"operation": "transaction", "action": "begin"},
                saved("b1", "active", "t1"),
                {"operation": "transaction", "action": "commit", "transaction": "t1"},
                {"operation": "transaction", "action": "begin"},
                saved("b2", "draft", "t2"),
                {"operation": "transaction", "action": "rollback", "transaction": "t2"},
            ]),
            "{report}"
        );
        assert_eq!(
            store["shelved"],
            json!({"value": {"code": "b1", "status": "active"}}),
            "{report}"
        );
        assert_eq!(store["refused"]["message"], "the shelf is full", "{report}");
    }

    fn assert_events(report: &serde_json::Value) {
        assert_eq!(
            report["events"],
            json!({
                "returned": ["early 25", "late 25"],
                "settled": ["early 25", "late 25", "background 25"],
                "started": ["start"],
                "stopped": ["start", "stop"],
                "failures": [],
            }),
            "{report}"
        );
    }

    async fn check(extension: &str) {
        if !NodeDriver::available() {
            eprintln!("skipping: node is unavailable");
            return;
        }
        let report = Self::report(extension).await;
        Self::assert_cache_and_resilience(&report);
        Self::assert_validation_and_settings(&report);
        Self::assert_store(&report);
        Self::assert_events(&report);
    }
}

#[tokio::test]
async fn typescript_behavior_store_and_event_markers_take_effect_in_built_services() {
    ScriptMarkerProofs::check("ts").await;
}

#[tokio::test]
async fn javascript_behavior_store_and_event_markers_take_effect_in_built_services() {
    ScriptMarkerProofs::check("js").await;
}
