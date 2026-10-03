#![allow(clippy::expect_used)]

use super::{BuildJobs, WebLanguage, build_all};
use std::path::{Path, PathBuf};
use std::process::Command;

struct FsharpSignal;

impl FsharpSignal {
    const SERVICE: &str = "client/services/SignalService.fs";
    const MODELS: [(&str, &str); 2] = [
        (
            "client/models/SignalSent.fs",
            "type SignalSent = { Size: int }\n",
        ),
        (
            "client/models/SignalEcho.fs",
            "type SignalEcho = { Size: int }\n",
        ),
    ];
    const CONSTRUCTED: &str = "open Models\n\n[<Service>]\n[<NeedsAll>]\ntype SignalService(applicationEventPublisher: ApplicationEventPublisher) =\n    member val Largest = 0 with get, set\n    member val Echoed = 0 with get, set\n    member val Started = false with get, set\n\n    \
             [<OnStart>]\n    member this.Start() : unit =\n        this.Started <- true\n\n    \
             member this.Send(size: int) : unit =\n        applicationEventPublisher.PublishEvent({ SignalSent.Size = size })\n\n    \
             [<OnEvent({\"condition\":\"value.size > 5\"})>]\n    member this.Record(value: SignalSent) : SignalEcho =\n        this.Largest <- max this.Largest value.Size\n        { SignalEcho.Size = value.Size * 10 }\n\n    \
             [<OnEvent>]\n    [<Order(1)>]\n    member this.Echo(value: SignalEcho) : unit =\n        this.Echoed <- value.Size\n\n    \
             [<OnStop>]\n    member this.Stop() : unit =\n        this.Started <- false\n";
    const NEEDS_PROPERTY: &str = "open Models\n\n[<Service>]\ntype SignalService() =\n    [<Needs>]\n    member val Publisher : ApplicationEventPublisher = Unchecked.defaultof<ApplicationEventPublisher> with get, set\n    member val Largest = 0 with get, set\n    member val Echoed = 0 with get, set\n    member val Started = false with get, set\n\n    \
             [<OnStart>]\n    member this.Start() : unit =\n        this.Started <- true\n\n    \
             member this.Send(size: int) : unit =\n        this.Publisher.PublishEvent({ SignalSent.Size = size })\n\n    \
             [<OnEvent({\"condition\":\"value.size > 5\"})>]\n    member this.Record(value: SignalSent) : SignalEcho =\n        this.Largest <- max this.Largest value.Size\n        { SignalEcho.Size = value.Size * 10 }\n\n    \
             [<OnEvent>]\n    [<Order(1)>]\n    member this.Echo(value: SignalEcho) : unit =\n        this.Echoed <- value.Size\n\n    \
             [<OnStop>]\n    member this.Stop() : unit =\n        this.Started <- false\n";

    const DRIVER: &str = "const { default: SignalService, __tachyon_event_plan__: plan } = await import('./SignalService.js')\n\
         const { default: ApplicationEvents } = await import('./application-events.mjs')\n\
         const events = []\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })\n\
         for (const event of ['SignalSent', 'SignalEcho']) bus.attach({ receive: (value) => events.push([event, value]) }, { listeners: [{ method: 'receive', event, order: -100 }] }, {})\n\
         const signal = new SignalService({})\n\
         bus.attach(signal, plan.types[0], { applicationEventPublisher: null })\n\
         await bus.initialize(signal)\n\
         signal.send(3); signal.send(8)\n\
         console.log(JSON.stringify({ largest: signal.largest, echoed: signal.echoed, started: signal.started, events }))\n\
         process.exit(0)\n";
}

pub(super) fn fsharp_web_ready() -> bool {
    let workloads = Command::new("dotnet").args(["workload", "list"]).output();
    let has_workload = workloads
        .is_ok_and(|output| String::from_utf8_lossy(&output.stdout).contains("wasm-tools"));
    has_workload && Command::new("node").arg("--version").output().is_ok()
}

fn write(root: &Path, relative: &str, text: &str) -> PathBuf {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
    std::fs::write(&path, text).expect("file");
    path
}

fn run_driver(root: &Path, script: &str) -> serde_json::Value {
    std::fs::write(root.join("drive.mjs"), script).expect("driver");
    let output = Command::new("node")
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
    serde_json::from_str(stdout.trim()).expect("report")
}

async fn publish_signal(root: &Path, service: &str) {
    for (relative, text) in FsharpSignal::MODELS {
        write(root, relative, text);
    }
    let source = write(root, FsharpSignal::SERVICE, service);
    let services = [(
        String::from("signal"),
        String::from("SignalService"),
        WebLanguage::CSharp,
        source,
    )];
    let built = build_all(
        root,
        &services,
        &BuildJobs::new(1),
        crate::BuildProfile::Release,
    )
    .await
    .expect("the F# signal service builds");
    for (name, bytes) in built.files {
        write(root, &name, "");
        std::fs::write(root.join(name), bytes).expect("publish");
    }
    std::fs::write(
        root.join("application-events.mjs"),
        concat!(
            include_str!("../template/application-events.js"),
            "\nexport default TacApplicationEvents\n"
        ),
    )
    .expect("event runtime");
}

async fn signal_report(service: &str) -> Option<serde_json::Value> {
    if !fsharp_web_ready() {
        eprintln!("skipping: the .NET wasm-tools workload or node is unavailable");
        return None;
    }
    let root = tempfile::tempdir().expect("project");
    publish_signal(root.path(), service).await;
    Some(run_driver(root.path(), FsharpSignal::DRIVER))
}

fn assert_signalled(report: &serde_json::Value) {
    assert_eq!(report["largest"], 8, "{report}");
    assert_eq!(report["echoed"], 80, "{report}");
    assert_eq!(report["started"], true, "{report}");
    assert_eq!(
        report["events"],
        serde_json::json!([
            ["SignalSent", {"size": 3}],
            ["SignalSent", {"size": 8}],
            ["SignalEcho", {"size": 80}],
        ])
    );
}

#[tokio::test]
async fn an_fsharp_service_publishes_and_listens_on_the_page_event_bus() {
    if let Some(report) = signal_report(FsharpSignal::CONSTRUCTED).await {
        assert_signalled(&report);
    }
}

#[tokio::test]
async fn an_fsharp_service_takes_its_publisher_as_a_needs_property() {
    if let Some(report) = signal_report(FsharpSignal::NEEDS_PROPERTY).await {
        assert_signalled(&report);
    }
}
