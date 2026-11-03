#![allow(clippy::expect_used)]

use super::java_tests::{java_web_ready, publish, run_driver, write};
use super::{BuildJobs, WebLanguage, build_all};
use std::path::Path;

struct JavaLifecycleFixtures;

impl JavaLifecycleFixtures {
    const FILES: [(&str, &str); 3] = [
        (
            "client/models/OrderPlaced.java",
            "record OrderPlaced(int total) {}\n",
        ),
        (
            "client/models/OrderSettled.java",
            "record OrderSettled(int total) {}\n",
        ),
        (
            "client/services/JournalService.java",
            r#"import java.util.ArrayList;
import java.util.List;
import models.OrderPlaced;
import models.OrderSettled;

@Service
@Description("Keeps the order journal")
class JournalService {
    static List<String> stopped = new ArrayList<>();
    List<String> heard = new ArrayList<>();
    ApplicationEventPublisher applicationEventPublisher;

    @OnStart
    void open() {
        heard.add("start");
    }

    @OnStop
    void close() {
        stopped.add("stop after " + heard.size());
    }

    void place(int total) {
        applicationEventPublisher.publishEvent(new OrderPlaced(total));
        heard.add("placed");
    }

    List<String> stops() {
        return stopped;
    }

    @OnEvent
    @Order(20)
    void late(OrderPlaced order) {
        heard.add("late " + order.total());
    }

    @OnEvent
    @Order(-5)
    OrderSettled early(OrderPlaced order) {
        heard.add("early " + order.total());
        return new OrderSettled(order.total() * 2);
    }

    @OnEvent
    @InBackground
    OrderSettled audit(OrderPlaced order) {
        heard.add("audit " + order.total());
        return new OrderSettled(-1);
    }

    @OnEvent
    void settled(OrderSettled order) {
        heard.add("settled " + order.total());
    }
}
"#,
        ),
    ];

    const DRIVER: &str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n\
         const { default: JournalService, __tachyon_event_plan__: plan } = await import('./JournalService.js')\n\
         const { default: ApplicationEvents } = await import('./application-events.mjs')\n\
         const events = []\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })\n\
         for (const event of ['OrderPlaced', 'OrderSettled']) bus.attach({ receive: (value) => events.push([event, value.total]) }, { listeners: [{ method: 'receive', event, order: 100 }] }, {})\n\
         const journal = new JournalService({})\n\
         bus.attach(journal, plan.types[0], { applicationEventPublisher: null })\n\
         await bus.initialize(journal)\n\
         journal.__tachyonStarted()\n\
         journal.place(3)\n\
         const before = [...journal.heard]\n\
         await new Promise((settle) => setTimeout(settle, 10))\n\
         const after = [...journal.heard]\n\
         const listeners = plan.types[0].listeners.map(({ method, order, asynchronous }) => [method.split('_').at(-1), order ?? 0, asynchronous === true])\n\
         journal.__tachyonDispose()\n\
         const witness = new JournalService({})\n\
         const stops = witness.stops()\n\
         console.log(JSON.stringify({ before, after, events, listeners, stops }))\n\
         process.exit(0)\n";

    async fn refusal(root: &Path) -> String {
        let services = [(
            String::from("journal"),
            String::from("JournalService"),
            WebLanguage::Java,
            root.join("client/services/JournalService.java"),
        )];
        let built = build_all(
            root,
            &services,
            &BuildJobs::new(1),
            crate::BuildProfile::Development,
        )
        .await;
        built
            .err()
            .expect("the Java service is refused")
            .to_string()
    }

    fn application_events() -> String {
        format!(
            "{}\nexport default TacApplicationEvents\n",
            include_str!("../template/application-events.js")
        )
    }
}

#[tokio::test]
async fn a_java_web_service_runs_lifecycle_hooks_and_ordered_listeners() {
    if !java_web_ready() {
        eprintln!("skipping: TeaVM (TAC_TEAVM_HOME), javac or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    for (relative, text) in JavaLifecycleFixtures::FILES {
        write(root.path(), relative, text);
    }
    write(
        root.path(),
        "application-events.mjs",
        &JavaLifecycleFixtures::application_events(),
    );
    publish(root.path(), "journal", "JournalService").await;
    let report = run_driver(root.path(), JavaLifecycleFixtures::DRIVER);
    assert_eq!(
        report["before"],
        serde_json::json!(["start", "placed", "early 3", "settled 6", "late 3"]),
        "{report}"
    );
    assert_eq!(
        report["after"],
        serde_json::json!([
            "start",
            "placed",
            "early 3",
            "settled 6",
            "late 3",
            "audit 3"
        ]),
        "{report}"
    );
    assert_eq!(
        report["events"],
        serde_json::json!([["OrderSettled", 6], ["OrderPlaced", 3]]),
        "{report}"
    );
    assert_eq!(
        report["listeners"],
        serde_json::json!([
            ["late", 20, false],
            ["early", -5, false],
            ["audit", 0, true],
            ["settled", 0, false],
        ]),
        "{report}"
    );
    assert_eq!(
        report["stops"],
        serde_json::json!(["stop after 6"]),
        "{report}"
    );
}

#[tokio::test]
async fn a_java_web_service_reads_its_description_as_behavior_metadata() {
    let root = tempfile::tempdir().expect("project");
    let [placed, settled, (relative, service)] = JavaLifecycleFixtures::FILES;
    for (relative, text) in [placed, settled] {
        write(root.path(), relative, text);
    }
    write(
        root.path(),
        relative,
        &service.replace(
            "@Description(\"Keeps the order journal\")",
            "@Description(\"\")",
        ),
    );
    let failure = JavaLifecycleFixtures::refusal(root.path()).await;
    assert!(
        failure.contains("Description requires one quoted line of at most 256 characters"),
        "{failure}"
    );
}
