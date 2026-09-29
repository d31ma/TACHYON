#![allow(clippy::expect_used)]

use tachyon_contracts::{HandlerRequest, HttpMethod};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
};

struct FsharpProbe;

impl FsharpProbe {
    const ENTRY: &str = "server/controllers/ProbeController.fs";
    const FILES: [(&str, &str); 8] = [
        (
            "server/models/ProbeReady.fs",
            "type ProbeReady = { Status: string; Total: int64 }\n",
        ),
        (
            "server/models/ProbeDerived.fs",
            "type ProbeDerived = { Tag: string }\n",
        ),
        (
            "server/models/ProbeTagged.fs",
            "type ProbeTagged = { Tag: string }\n",
        ),
        (
            "server/models/ProbeReport.fs",
            "type ProbeReport = { Log: string list; Tag: string }\n",
        ),
        (
            "server/services/ProbeService.fs",
            "open Models\n\n[<Service>]\ntype ProbeService() =\n    member val Log : ResizeArray<string> = ResizeArray<string>() with get\n    member val Tag = \"\" with get, set\n\n    [<OnEvent>]\n    [<Order(2)>]\n    member this.Wide(value: ProbeReady) : unit =\n        this.Log.Add(\"wide:\" + value.Status)\n\n    [<OnEvent({\"condition\":\"value.status == 'paid' && value.total >= 10\"})>]\n    [<Order(1)>]\n    member this.Paid(value: ProbeReady) : ProbeDerived =\n        this.Log.Add(\"paid\")\n        { ProbeDerived.Tag = \"one\" }\n\n    [<OnEvent({\"condition\":\"value.total > 100\"})>]\n    [<Order(0)>]\n    member this.Skipped(value: ProbeReady) : unit =\n        this.Log.Add(\"skipped\")\n\n    [<OnEvent>]\n    member this.Derived(value: ProbeDerived) : unit =\n        this.Log.Add(\"derived:\" + value.Tag)\n\n    [<OnEvent({\"replay\":true})>]\n    member this.Tagged(value: ProbeTagged) : unit =\n        this.Tag <- value.Tag\n",
        ),
        (
            "server/services/PublishService.fs",
            "open Models\n\n[<Service>]\n[<NeedsAll>]\ntype PublishService(publisher: ApplicationEventPublisher) =\n    [<OnStart>]\n    member this.Initialize() : unit =\n        publisher.PublishEvent({ Status = \"paid\"; Total = 10L }, true) |> ignore\n        publisher.publishEvent({ Status = \"free\"; Total = 0L }) |> ignore\n",
        ),
        (
            "server/services/ResourceService.fs",
            "open Models\n\n[<Service>]\ntype ResourceService(applicationEventPublisher: ApplicationEventPublisher) =\n    [<OnStart>]\n    member this.Initialize() : unit =\n        applicationEventPublisher.PublishEvent({ ProbeTagged.Tag = \"resource\" }) |> ignore\n        applicationEventPublisher.PublishEvent({ ProbeTagged.Tag = null }) |> ignore\n\n    [<OnStop>]\n    member this.Stop() : unit = ()\n",
        ),
        (
            "server/controllers/ProbeController.fs",
            "open Models\n\n[<RequestRoute(\"/probe\")>]\n[<Public>]\ntype ProbeController(probe: ProbeService, publish: PublishService, resource: ResourceService) =\n    [<GetRoute>]\n    member this.GetProbe() : ProbeReport =\n        { Log = List.ofSeq probe.Log; Tag = probe.Tag }\n",
        ),
    ];
}

#[tokio::test]
async fn fsharp_events_and_lifecycle_run_through_the_csharp_runtime() {
    if std::process::Command::new("dotnet")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let project = tempfile::tempdir().expect("project");
    for (relative, source) in FsharpProbe::FILES {
        let target = project.path().join(relative);
        std::fs::create_dir_all(target.parent().expect("parent")).expect("folders");
        std::fs::write(target, source).expect("source");
    }
    let source = HandlerSource::discover(project.path(), FsharpProbe::ENTRY)
        .unwrap_or_else(|failure| panic!("F# discovery: {failure}"));
    let supervisor =
        HandlerSupervisor::new(HandlerSupervisorOptions::default()).expect("supervisor");
    for attempt in 0..2 {
        let request = HandlerRequest::route(
            format!("fsharp_events_{attempt}"),
            "/probe",
            HttpMethod::Get,
        );
        let response = supervisor
            .invoke(&source, &request, &HandlerCancellation::default())
            .await
            .expect("F# invocation");
        assert_eq!(response.status, 200, "{:?}", response.error);
        let body: serde_json::Value =
            serde_json::from_str(&response.body.expect("body").data).expect("JSON");
        assert_eq!(
            body,
            serde_json::json!({
                "log": ["paid", "wide:paid", "wide:free", "derived:one"],
                "tag": "resource",
            })
        );
    }
}
