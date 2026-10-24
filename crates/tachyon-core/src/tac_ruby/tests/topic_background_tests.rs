use super::{Page, write};
use std::path::Path;

struct TopicBackgroundPage;

impl TopicBackgroundPage {
    const FILES: [(&'static str, &'static str); 3] = [
        (
            "client/models/InventoryRecord.rb",
            "#[Topic]\n# @!attribute [r] sku\n#   @return [String]\n# @!attribute [r] level\n#   @return [Integer]\nInventoryRecord = Data.define(:sku, :level)\n",
        ),
        (
            "client/models/OrderPlaced.rb",
            "# @!attribute [r] total\n#   @return [Integer]\nOrderPlaced = Data.define(:total)\n",
        ),
        (
            "client/models/OrderSettled.rb",
            "# @!attribute [r] total\n#   @return [Integer]\nOrderSettled = Data.define(:total)\n",
        ),
    ];

    const INVENTORY: &'static str = "#[Service]\nclass InventoryService\n  # @return [Array<InventoryRecord>]\n  attr_reader :records\n\n  def initialize\n    @records = []\n  end\n\n  # @param record [InventoryRecord]\n  # @return [void]\n  #[OnEvent]\n  def add_record(record)\n    @records = [*@records.last(63), record]\n  end\nend\n";

    const JOURNAL: &'static str = "#[Service]\nclass JournalService\n  attr_reader :heard\n\n  def initialize\n    @heard = []\n    @events = ApplicationEventPublisher.new\n  end\n\n  def place(total)\n    @events.publish_event(OrderPlaced.new(total: total))\n    @heard << \"placed\"\n  end\n\n  # @param order [OrderPlaced]\n  #[OnEvent]\n  def record(order)\n    @heard << \"record #{order.total}\"\n  end\n\n  # @param order [OrderPlaced]\n  # @return [OrderSettled]\n  #[OnEvent]\n  #[InBackground]\n  def audit(order)\n    @heard << \"audit #{order.total}\"\n    OrderSettled.new(total: -1)\n  end\n\n  # @param order [OrderSettled]\n  #[OnEvent]\n  def settled(order)\n    @heard << \"settled #{order.total}\"\n  end\nend\n";

    const TOPIC_DRIVER: &'static str = "globalThis.location = { href: 'http://127.0.0.1:4000/page' }\n\
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

    const BACKGROUND_DRIVER: &'static str = "const { default: JournalService, __tachyon_event_plan__: plan } = await import('./JournalService.js')\n\
         const { default: ApplicationEvents } = await import('./application-events.mjs')\n\
         const events = []\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })\n\
         for (const event of ['OrderPlaced', 'OrderSettled']) bus.attach({ receive: (value) => events.push([event, value.total]) }, { listeners: [{ method: 'receive', event, order: 100 }] }, {})\n\
         const journal = new JournalService({})\n\
         bus.attach(journal, plan.types[0], { applicationEventPublisher: null })\n\
         await bus.initialize(journal)\n\
         journal.place(3)\n\
         const before = [...journal.heard]\n\
         await new Promise((settle) => setTimeout(settle, 10))\n\
         const after = [...journal.heard]\n\
         const listeners = plan.types[0].listeners.map(({ method, asynchronous }) => [method, asynchronous === true])\n\
         console.log(JSON.stringify({ before, after, events, listeners }))\n\
         process.exit(0)\n";

    async fn publish(root: &Path, service: (&str, &str, &str)) {
        for (relative, text) in Self::FILES {
            write(root, relative, text);
        }
        Page::publish(root, &[service]).await;
        write(
            root,
            "application-events.mjs",
            concat!(
                include_str!("../../template/application-events.js"),
                "\nexport default TacApplicationEvents\n"
            ),
        );
    }
}

#[tokio::test]
async fn a_ruby_topic_model_reaches_its_listener_from_its_topic_stream_on_the_page() {
    if let Some(reason) = Page::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    TopicBackgroundPage::publish(
        root.path(),
        (
            "inventory",
            "InventoryService",
            TopicBackgroundPage::INVENTORY,
        ),
    )
    .await;
    let report = Page::run(root.path(), TopicBackgroundPage::TOPIC_DRIVER);
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

#[tokio::test]
async fn a_ruby_in_background_listener_runs_after_the_publisher_returns() {
    if let Some(reason) = Page::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    TopicBackgroundPage::publish(
        root.path(),
        ("journal", "JournalService", TopicBackgroundPage::JOURNAL),
    )
    .await;
    let report = Page::run(root.path(), TopicBackgroundPage::BACKGROUND_DRIVER);
    assert_eq!(
        report["before"],
        serde_json::json!(["placed", "record 3"]),
        "{report}"
    );
    assert_eq!(
        report["after"],
        serde_json::json!(["placed", "record 3", "audit 3"]),
        "{report}"
    );
    assert_eq!(
        report["events"],
        serde_json::json!([["OrderPlaced", 3]]),
        "{report}"
    );
    assert_eq!(
        report["listeners"],
        serde_json::json!([
            ["__tachyonListen:record", false],
            ["__tachyonListen:audit", true],
            ["__tachyonListen:settled", false]
        ]),
        "{report}"
    );
}
