#![allow(clippy::expect_used)]

use super::fsharp_tests::{fsharp_web_ready, publish, run_driver, write};
use super::{BuildJobs, WebLanguage, build_all};

struct FsharpLifecycle;

impl FsharpLifecycle {
    const SERVICE: &str = "client/services/ShelfService.fs";
    const FILES: [(&str, &str); 3] = [
        (
            "client/models/TickSent.fs",
            "type TickSent = { Size: int }\n",
        ),
        (
            "client/models/ShelfChanged.fs",
            "[<Topic>]\ntype ShelfChanged = { Count: int }\n",
        ),
        (
            Self::SERVICE,
            "open Models\n\n[<Service>]\n[<NeedsAll>]\n[<Description(\"Shelf lifecycle, listener order and topic proof\")>]\ntype ShelfService(label: string, count: int, applicationEventPublisher: ApplicationEventPublisher) =\n    static let mutable stops = 0\n\n    \
             member val Title = label with get, set\n    member val Count = count with get, set\n    member val Log = \"\" with get, set\n    member val Shelf = 0 with get, set\n\n    \
             member this.Stops() : int = stops\n\n    \
             member this.Tick(size: int) : unit =\n        applicationEventPublisher.PublishEvent({ TickSent.Size = size })\n\n    \
             [<OnStart>]\n    member this.Start() : unit =\n        this.Log <- this.Log + \"start;\"\n\n    \
             [<OnEvent>]\n    [<InBackground>]\n    [<Order(-5)>]\n    member this.Later(value: TickSent) : unit =\n        this.Log <- this.Log + \"background;\"\n\n    \
             [<OnEvent>]\n    [<Order(2)>]\n    member this.Second(value: TickSent) : unit =\n        this.Log <- this.Log + \"second;\"\n\n    \
             [<OnEvent>]\n    [<Order(1)>]\n    member this.First(value: TickSent) : unit =\n        this.Log <- this.Log + \"first;\"\n\n    \
             [<OnEvent>]\n    member this.Restock(value: ShelfChanged) : unit =\n        this.Shelf <- value.Count\n\n    \
             [<OnStop>]\n    member this.Stop() : unit =\n        stops <- stops + 1\n",
        ),
    ];

    const DRIVER: &str = "const sources = []\n\
         globalThis.EventSource = class { constructor(url) { this.url = url; sources.push(this) } addEventListener() {} close() {} }\n\
         const { default: ShelfService, __tachyon_event_plan__: plan } = await import('./ShelfService.js')\n\
         const { default: ApplicationEvents } = await import('./application-events.mjs')\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })\n\
         const shelf = new ShelfService({ label: 'pantry', count: 4 })\n\
         const witness = new ShelfService({ label: 'witness', count: 0 })\n\
         for (const owner of [shelf, witness]) { bus.attach(owner, plan.types[0], { applicationEventPublisher: null }); await bus.initialize(owner) }\n\
         shelf.tick(3)\n\
         const settled = shelf.log\n\
         await new Promise((resolve) => setTimeout(resolve, 0))\n\
         for (const source of sources) source.onmessage({ data: JSON.stringify({ count: 7 }), lastEventId: '1' })\n\
         const report = { title: shelf.title, count: shelf.count, settled, log: shelf.log, shelf: shelf.shelf, topics: sources.map((source) => source.url), stopsBefore: witness.stops() }\n\
         bus.dispose(shelf)\n\
         console.log(JSON.stringify({ ...report, stopsAfter: witness.stops() }))\n\
         process.exit(0)\n";

    const STORE: (&str, &str) = (
        "client/stores/NoteStore.fs",
        "[<Store>]\ntype NoteStore() =\n    member this.Count() : int = 0\n",
    );
    const NEEDS: [&str; 2] = ["Clock", "NoteStore"];

    async fn refusal(needed: &str) -> String {
        let root = tempfile::tempdir().expect("project");
        let (store, text) = Self::STORE;
        write(root.path(), store, text);
        let service = format!(
            "[<Service>]\ntype ClockService() =\n    [<Needs>]\n    member val Needed : {needed} = Unchecked.defaultof<{needed}> with get, set\n"
        );
        let source = write(root.path(), "client/services/ClockService.fs", &service);
        let services = [(
            String::from("clock"),
            String::from("ClockService"),
            WebLanguage::CSharp,
            source,
        )];
        let Err(failure) = build_all(
            root.path(),
            &services,
            &BuildJobs::new(1),
            crate::BuildProfile::Development,
        )
        .await
        else {
            panic!("an F# Tac Needs of a browser API or a Store is refused");
        };
        failure.to_string()
    }
}

#[tokio::test]
async fn an_fsharp_service_runs_its_lifecycle_listener_order_props_and_topic() {
    if !fsharp_web_ready() {
        eprintln!("skipping: the .NET wasm-tools workload or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    publish(
        root.path(),
        &FsharpLifecycle::FILES,
        ("ShelfService", FsharpLifecycle::SERVICE),
    )
    .await;
    let report = run_driver(root.path(), FsharpLifecycle::DRIVER);
    assert_eq!(
        report,
        serde_json::json!({
            "title": "pantry",
            "count": 4,
            "settled": "start;first;second;",
            "log": "start;first;second;background;",
            "shelf": 7,
            "topics": ["/.tachyon/topics/shelf-changed"],
            "stopsBefore": 0,
            "stopsAfter": 1,
        }),
        "NeedsAll props arrive, Order runs listeners in order, InBackground runs last, the Topic stream reaches OnEvent and OnStop runs on dispose"
    );
}

#[tokio::test]
async fn an_fsharp_tac_service_cannot_need_a_browser_api_or_a_store() {
    for needed in FsharpLifecycle::NEEDS {
        let text = FsharpLifecycle::refusal(needed).await;
        assert!(text.contains("TY1405"), "{text}");
        assert!(text.contains("ADR 0073"), "{text}");
    }
}
