use super::rust_marker_proof_tests::RustPage;

struct RustTopicPage;

impl RustTopicPage {
    const FILES: [(&'static str, &'static str); 2] = [
        (
            "client/models/InventoryRecord.rs",
            "#[Topic]\n#[derive(Clone, Debug, Default)]\npub struct InventoryRecord {\n    pub sku: String,\n    pub level: i64,\n}\n",
        ),
        (
            "client/services/InventoryService.rs",
            "use crate::models::InventoryRecord;\n\n#[Service]\n#[derive(Default)]\npub struct InventoryService {\n    pub records: Vec<InventoryRecord>,\n}\n\n\
             impl InventoryService {\n    #[OnEvent]\n    fn add_record(&mut self, record: InventoryRecord) {\n        self.records.push(record);\n    }\n}\n",
        ),
    ];

    const DRIVER: &'static str = "import { readFile } from 'node:fs/promises'\n\
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
}

#[tokio::test]
async fn a_rust_topic_model_reaches_its_listener_from_its_topic_stream_on_the_page() {
    if !RustPage::ready() {
        eprintln!("skipping: the wasm32 Rust target or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().unwrap_or_else(|failure| panic!("project: {failure}"));
    let service = ("inventory", "InventoryService");
    RustPage::publish(root.path(), &RustTopicPage::FILES, service).await;
    let report = RustPage::drive(root.path(), RustTopicPage::DRIVER);
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
