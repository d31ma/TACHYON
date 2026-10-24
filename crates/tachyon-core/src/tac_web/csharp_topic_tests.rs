#![allow(clippy::expect_used)]

use super::fsharp_tests::fsharp_web_ready;
use super::{BuildJobs, WebLanguage, build_all};
use crate::node_driver::NodeDriver;
use std::path::{Path, PathBuf};

struct CsharpTopicPage;

impl CsharpTopicPage {
    const MODEL: (&str, &str) = (
        "client/models/InventoryRecord.cs",
        "[Topic]\npublic record InventoryRecord(string Sku, long Level);\n",
    );

    const SERVICE: (&str, &str) = (
        "client/services/InventoryService.cs",
        "using System;\nusing System.Collections.Generic;\nusing System.Linq;\nusing Models;\n\n\
         [Service]\npublic class InventoryService\n{\n    public List<InventoryRecord> Records = new List<InventoryRecord>();\n\n    \
         [OnEvent]\n    public void AddRecord(InventoryRecord record)\n    {\n        \
         var older = Math.Max(0, Records.Count - 63);\n        \
         Records = Records.Skip(older).Append(record).ToList();\n    }\n}\n",
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

    fn write(root: &Path, relative: &str, bytes: &[u8]) -> PathBuf {
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
            WebLanguage::CSharp,
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
async fn a_csharp_topic_model_reaches_its_listener_from_its_topic_stream_on_the_page() {
    if !fsharp_web_ready() {
        assert!(
            std::env::var_os("CI").is_none(),
            "CI must provide the .NET wasm-tools workload and node"
        );
        eprintln!("skipping: the .NET wasm-tools workload or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    CsharpTopicPage::publish(root.path()).await;
    let report = CsharpTopicPage::drive(root.path());
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
