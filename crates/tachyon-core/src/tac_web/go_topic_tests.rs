#![allow(clippy::expect_used)]

use super::{BuildJobs, WebLanguage, build_all};
use crate::node_driver::NodeDriver;
use std::path::Path;
use std::process::Command;

struct GoTopicPage;

impl GoTopicPage {
    const MODEL: (&str, &str) = (
        "client/models/InventoryRecord.go",
        "// @Topic\ntype InventoryRecord struct {\n\tSku   string `json:\"sku\"`\n\tLevel int64  `json:\"level\"`\n}\n",
    );

    const SERVICE: (&str, &str) = (
        "client/services/InventoryService.go",
        "// @Service\ntype InventoryService struct {\n\tRecords []InventoryRecord\n}\n\n\
         // @OnEvent\nfunc (inventory *InventoryService) AddRecord(record InventoryRecord) {\n\tinventory.Records = append(inventory.Records, record)\n}\n",
    );

    const DRIVER: &str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n\
         globalThis.location = { href: 'http://127.0.0.1:4000/page' }\n\
         const sources = []\n\
         globalThis.EventSource = class { constructor(url) { this.url = String(url); this.closed = false; sources.push(this) } addEventListener() {} close() { this.closed = true } }\n\
         const { default: InventoryService, __tachyon_event_plan__: plan } = await import('./InventoryService.js')\n\
         const { default: ApplicationEvents } = await import('./application-events.mjs')\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })\n\
         const inventory = new InventoryService({})\n\
         bus.attach(inventory, plan.types[0], { applicationEventPublisher: null })\n\
         await bus.initialize(inventory)\n\
         const before = inventory.records\n\
         for (const source of sources) source.onmessage({ data: JSON.stringify({ sku: 'anvil', level: 3 }), lastEventId: '1' })\n\
         for (const source of sources) source.onmessage({ data: JSON.stringify({ sku: 'rope', level: 7 }), lastEventId: '2' })\n\
         const report = { topics: sources.map((source) => source.url), before, records: inventory.records }\n\
         bus.dispose(inventory)\n\
         await new Promise((resolve) => setTimeout(resolve, 50))\n\
         report.closed = sources.map((source) => source.closed)\n\
         console.log(JSON.stringify(report))\n\
         process.exit(0)\n";

    fn tools_available() -> bool {
        [("go", "version"), ("node", "--version")]
            .iter()
            .all(|(tool, argument)| {
                Command::new(tool)
                    .arg(argument)
                    .output()
                    .is_ok_and(|output| output.status.success())
            })
    }

    fn write(root: &Path, relative: &str, bytes: &[u8]) -> std::path::PathBuf {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("folder")).expect("folder");
        std::fs::write(&path, bytes).expect("file");
        path
    }

    async fn publish(root: &Path) {
        let (model, model_text) = Self::MODEL;
        Self::write(root, model, model_text.as_bytes());
        let (service, service_text) = Self::SERVICE;
        let path = Self::write(root, service, service_text.as_bytes());
        let services = [(
            String::from("inventory"),
            String::from("InventoryService"),
            WebLanguage::Go,
            path,
        )];
        let built = build_all(
            root,
            &services,
            &BuildJobs::new(1),
            crate::BuildProfile::Development,
        )
        .await
        .unwrap_or_else(|failure| panic!("{failure}"));
        for (name, bytes) in built.files {
            Self::write(root, &name, &bytes);
        }
        Self::write(
            root,
            "application-events.mjs",
            concat!(
                include_str!("../template/application-events.js"),
                "\nexport default TacApplicationEvents\n"
            )
            .as_bytes(),
        );
    }

    fn drive(root: &Path) -> serde_json::Value {
        Self::write(root, "drive.mjs", Self::DRIVER.as_bytes());
        let output = NodeDriver::output(NodeDriver::command().arg("drive.mjs").current_dir(root))
            .expect("node");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "{stdout}{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_str(stdout.trim()).expect("report")
    }
}

#[tokio::test]
async fn a_go_topic_model_reaches_its_listener_from_its_topic_stream_on_the_page() {
    if !GoTopicPage::tools_available() {
        assert!(
            std::env::var_os("CI").is_none(),
            "CI must provide go and node"
        );
        eprintln!("skipping: go or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    GoTopicPage::publish(root.path()).await;
    let report = GoTopicPage::drive(root.path());
    assert_eq!(
        report,
        serde_json::json!({
            "topics": ["/.tachyon/topics/inventory-record"],
            "before": [],
            "records": [{"sku": "anvil", "level": 3}, {"sku": "rope", "level": 7}],
            "closed": [true],
        }),
        "{report}"
    );
}
