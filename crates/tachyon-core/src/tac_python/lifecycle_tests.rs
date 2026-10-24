use super::page_tests::PythonPage;
use std::path::Path;

struct PythonLifecycle;

impl PythonLifecycle {
    const LABEL: [(&'static str, &'static str); 1] = [(
        "client/helpers/TallyHelper.py",
        "@helper\nclass TallyHelper:\n    def start(self) -> int:\n        return 3\n",
    )];

    const LABEL_SERVICE: &'static str = "@service\n@needs_all\nclass LabelService:\n    tally: TallyHelper\n    title: str\n    start_at: int\n    clicks: int = 0\n\n    \
         def shown(self) -> str:\n        return self.title + ':' + str(self.start_at + self.tally.start() + self.clicks)\n";

    const LABEL_DRIVER: &'static str = "console.info = () => {}\n\
         const { default: LabelService } = await import('./LabelService.js')\n\
         const first = new LabelService({ title: 'A', startAt: 1, clicks: 9 })\n\
         const second = new LabelService({ title: 'B', startAt: 10 })\n\
         const shown = [first.shown(), second.shown()]\n\
         const hidden = first.tally === undefined\n\
         let missing = ''\n\
         try { new LabelService({ title: 'C' }) } catch (error) { missing = String(error.message) }\n\
         console.log(JSON.stringify({ shown, hidden, missing }))\n\
         process.exit(0)\n";

    const AUDIT: [(&'static str, &'static str); 2] = [
        (
            "client/helpers/AuditHelper.py",
            "@helper\nclass AuditHelper:\n    _start_count: int = 0\n    _stop_count: int = 0\n\n    def record_start(self) -> None:\n        self._start_count += 1\n\n    def record_stop(self) -> None:\n        self._stop_count += 1\n\n    def snapshot(self) -> list[int]:\n        return [self._start_count, self._stop_count]\n",
        ),
        (
            "client/services/AuditService.py",
            "from typing import Annotated\n\n@service\nclass AuditService:\n    audit: Annotated[AuditHelper, needs]\n\n    def snapshot(self) -> list[int]:\n        return self.audit.snapshot()\n",
        ),
    ];

    const TIMER_SERVICE: &'static str = "from typing import Annotated\n\n@service\nclass TimerService:\n    audit: Annotated[AuditHelper, needs]\n\n    \
         @on_start\n    def _start(self) -> None:\n        self.audit.record_start()\n\n    \
         @on_stop\n    def _stop(self) -> None:\n        self.audit.record_stop()\n\n    \
         def ping(self) -> str:\n        return 'pong'\n";

    const TIMER_DRIVER: &'static str = "console.info = () => {}\n\
         const { default: AuditService } = await import('./AuditService.js')\n\
         const { default: TimerService } = await import('./TimerService.js')\n\
         const audit = new AuditService({})\n\
         const before = audit.snapshot()\n\
         const first = new TimerService({})\n\
         const second = new TimerService({})\n\
         const started = audit.snapshot()\n\
         const pinged = first.ping()\n\
         first.__tachyonDispose()\n\
         const held = audit.snapshot()\n\
         second.__tachyonDispose()\n\
         const stopped = audit.snapshot()\n\
         console.log(JSON.stringify({ before, started, pinged, held, stopped }))\n\
         process.exit(0)\n";

    const TALLY_SERVICE: &'static str = "@service\n@lifetime({\"value\":\"prototype\"})\nclass TallyService:\n    count: int = 0\n\n    \
         def add(self) -> int:\n        self.count += 1\n        return self.count\n";

    const LEDGER_SERVICE: &'static str = "@service\nclass LedgerService:\n    count: int = 0\n\n    \
         def add(self) -> int:\n        self.count += 1\n        return self.count\n";

    const TALLY_DRIVER: &'static str = "console.info = () => {}\n\
         const { default: TallyService } = await import('./TallyService.js')\n\
         const { default: LedgerService } = await import('./LedgerService.js')\n\
         const tallies = [new TallyService({}), new TallyService({})]\n\
         const ledgers = [new LedgerService({}), new LedgerService({})]\n\
         tallies[0].add(); tallies[0].add(); ledgers[0].add()\n\
         console.log(JSON.stringify({ tallies: tallies.map((tally) => tally.count), ledgers: ledgers.map((ledger) => ledger.count) }))\n\
         process.exit(0)\n";

    const ORDERS: [(&'static str, &'static str); 2] = [
        (
            "client/models/PlacedOrder.py",
            "from dataclasses import dataclass\n\n\n@dataclass\nclass PlacedOrder:\n    status: str\n    total: int\n",
        ),
        (
            "client/models/SettledOrder.py",
            "from dataclasses import dataclass\n\n\n@dataclass\nclass SettledOrder:\n    total: int\n",
        ),
    ];

    const CHECKOUT_SERVICE: &'static str = "from models import PlacedOrder, SettledOrder\n\n\n@service\nclass CheckoutService:\n    settled: int = 0\n    audited: int = 0\n\n    \
         @on_event\n    def settle(self, order: PlacedOrder) -> SettledOrder:\n        self.settled = order.total\n        return SettledOrder(order.total)\n\n    \
         @on_event\n    @in_background\n    def audit(self, order: PlacedOrder) -> SettledOrder:\n        self.audited = order.total\n        return SettledOrder(order.total * 100)\n";

    const CHECKOUT_DRIVER: &'static str = "console.info = () => {}\n\
         const { default: CheckoutService, __tachyon_event_plan__: plan } = await import('./CheckoutService.js')\n\
         const { default: ApplicationEvents } = await import('./application-events.mjs')\n\
         const republished = []\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })\n\
         bus.attach({ receive: (value) => republished.push(value.total) }, { listeners: [{ method: 'receive', event: 'SettledOrder' }] }, {})\n\
         const checkout = new CheckoutService({})\n\
         bus.attach(checkout, plan.types[0], { applicationEventPublisher: null })\n\
         await bus.initialize(checkout)\n\
         bus.publish('PlacedOrder', { status: 'paid', total: 25 })\n\
         const during = { settled: checkout.settled, audited: checkout.audited }\n\
         await new Promise((resolve) => setTimeout(resolve, 20))\n\
         const after = { settled: checkout.settled, audited: checkout.audited }\n\
         console.log(JSON.stringify({ during, after, republished }))\n\
         process.exit(0)\n";

    fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
        for (relative, text) in files {
            PythonPage::write(root.path(), relative, text);
        }
        root
    }

    fn with_events(root: &Path) {
        PythonPage::write(
            root,
            "application-events.mjs",
            concat!(
                include_str!("../template/application-events.js"),
                "\nexport default TacApplicationEvents\n"
            ),
        );
    }
}

#[tokio::test]
async fn a_python_needs_all_service_takes_its_properties_and_its_helper() {
    if let Some(reason) = PythonPage::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = PythonLifecycle::project(&PythonLifecycle::LABEL);
    PythonPage::publish(
        root.path(),
        &[("label", "LabelService", PythonLifecycle::LABEL_SERVICE)],
    )
    .await;
    let report = PythonPage::run(root.path(), PythonLifecycle::LABEL_DRIVER);
    assert_eq!(
        report["shown"],
        serde_json::json!(["A:4", "B:13"]),
        "{report}"
    );
    assert_eq!(report["hidden"], true, "{report}");
    assert_eq!(
        report["missing"], "Web service label: Python Tac operation failed.",
        "{report}"
    );
}

#[tokio::test]
async fn a_python_stop_hook_runs_when_the_last_owner_of_its_service_is_disposed() {
    if let Some(reason) = PythonPage::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = PythonLifecycle::project(&PythonLifecycle::AUDIT[..1]);
    let (_, audit) = PythonLifecycle::AUDIT[1];
    PythonPage::publish(
        root.path(),
        &[
            ("audit", "AuditService", audit),
            ("timer", "TimerService", PythonLifecycle::TIMER_SERVICE),
        ],
    )
    .await;
    let report = PythonPage::run(root.path(), PythonLifecycle::TIMER_DRIVER);
    assert_eq!(report["before"], serde_json::json!([0, 0]), "{report}");
    assert_eq!(report["started"], serde_json::json!([1, 0]), "{report}");
    assert_eq!(report["pinged"], "pong", "{report}");
    assert_eq!(report["held"], serde_json::json!([1, 0]), "{report}");
    assert_eq!(report["stopped"], serde_json::json!([1, 1]), "{report}");
}

#[tokio::test]
async fn a_python_prototype_service_gives_each_owner_its_own_object() {
    if let Some(reason) = PythonPage::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = PythonLifecycle::project(&[]);
    PythonPage::publish(
        root.path(),
        &[
            ("tally", "TallyService", PythonLifecycle::TALLY_SERVICE),
            ("ledger", "LedgerService", PythonLifecycle::LEDGER_SERVICE),
        ],
    )
    .await;
    let report = PythonPage::run(root.path(), PythonLifecycle::TALLY_DRIVER);
    assert_eq!(report["tallies"], serde_json::json!([2, 0]), "{report}");
    assert_eq!(report["ledgers"], serde_json::json!([1, 1]), "{report}");
}

#[tokio::test]
async fn a_python_in_background_listener_runs_after_the_publisher_and_never_republishes() {
    if let Some(reason) = PythonPage::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = PythonLifecycle::project(&PythonLifecycle::ORDERS);
    PythonPage::publish(
        root.path(),
        &[(
            "checkout",
            "CheckoutService",
            PythonLifecycle::CHECKOUT_SERVICE,
        )],
    )
    .await;
    PythonLifecycle::with_events(root.path());
    let report = PythonPage::run(root.path(), PythonLifecycle::CHECKOUT_DRIVER);
    assert_eq!(
        report["during"],
        serde_json::json!({"settled": 25, "audited": 0}),
        "{report}"
    );
    assert_eq!(
        report["after"],
        serde_json::json!({"settled": 25, "audited": 25}),
        "{report}"
    );
    assert_eq!(report["republished"], serde_json::json!([25]), "{report}");
}
