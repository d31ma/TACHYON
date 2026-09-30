#![allow(clippy::expect_used)]

use tachyon_contracts::{HandlerRequest, HttpMethod};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
};

struct FsharpNeeds;

impl FsharpNeeds {
    const ENTRY: &str = "server/controllers/StockController.fs";
    const FILES: [(&str, &str); 5] = [
        (
            "server/models/Stock.fs",
            "type Stock = { Name: string; Count: int }\n",
        ),
        (
            "server/services/NameService.fs",
            "[<Service>]\ntype NameService() =\n    member this.First() : string = \"anvil\"\n",
        ),
        (
            "server/services/CountService.fs",
            "[<Service>]\ntype CountService() =\n    abstract Count : unit -> int\n    default this.Count() : int = 1\n",
        ),
        (
            "server/services/FastCountService.fs",
            "[<Service>]\ntype FastCountService() =\n    inherit CountService()\n    override this.Count() : int = 9\n",
        ),
        (
            "server/services/StockService.fs",
            "[<Service>]\ntype StockService() =\n    [<Needs>]\n    member val Names : NameService = Unchecked.defaultof<NameService> with get, set\n\n    member this.Name() : string = this.Names.First()\n",
        ),
    ];
    const CONTROLLER: &str = "open Models\n\n[<RequestRoute(\"/stock\")>]\n[<Public>]\ntype StockController(stock: StockService) =\n    [<Needs>]\n    [<Named(\"FastCountService\")>]\n    member val Counter : CountService = Unchecked.defaultof<CountService> with get, set\n\n    [<GetRoute>]\n    member this.GetStock() : Stock =\n        { Name = stock.Name(); Count = this.Counter.Count() }\n";
    const TWICE: &str = "open Models\n\n[<RequestRoute(\"/stock\")>]\n[<Public>]\ntype StockController(stock: StockService) =\n    [<Needs>]\n    member val Stock : StockService = Unchecked.defaultof<StockService> with get, set\n\n    [<GetRoute>]\n    member this.GetStock() : Stock =\n        { Name = stock.Name(); Count = 1 }\n";
    const SETUP: &str = "[<TestSetup>]\ntype StockTestSetup() =\n    static member configure(execution: BehaviorExecution) : BehaviorAdapters =\n        BehaviorAdapters()\n";
    const UNIT: &str = "[<Test(typeof<StockTestSetup>)>]\ntype StockServiceTest() =\n    [<Fake>]\n    member val Names : NameService = Unchecked.defaultof<NameService> with get, set\n\n    member this.testReadsTheFakeName() : unit =\n        this.Names <- NameService()\n        let service = StockService(Names = this.Names)\n        Assert.equals(\"anvil\", service.Name())\n\n    member this.testStartsWithoutAFake() : unit =\n        Assert.isTrue(isNull (box this.Names), \"the fake is cleared\")\n";

    fn dotnet_available() -> bool {
        std::process::Command::new("dotnet")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
    }

    fn project(controller: &str) -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        let files = Self::FILES.into_iter().chain([
            (Self::ENTRY, controller),
            ("server/tests/StockTestSetup.fs", Self::SETUP),
            ("server/tests/StockServiceTest.fs", Self::UNIT),
        ]);
        for (relative, source) in files {
            let target = root.path().join(relative);
            std::fs::create_dir_all(target.parent().expect("parent")).expect("folders");
            std::fs::write(target, source).expect("source");
        }
        root
    }

    async fn invoke(
        root: &std::path::Path,
        entry: &str,
        request: &HandlerRequest,
    ) -> serde_json::Value {
        let source = HandlerSource::discover(root, entry)
            .unwrap_or_else(|failure| panic!("F# discovery: {failure}"));
        let supervisor =
            HandlerSupervisor::new(HandlerSupervisorOptions::default()).expect("supervisor");
        let response = supervisor
            .invoke(&source, request, &HandlerCancellation::default())
            .await
            .unwrap_or_else(|error| panic!("F# invocation: {error:?}"));
        assert_eq!(response.status, 200, "{:?}", response.error);
        serde_json::from_str(&response.body.expect("body").data).expect("JSON")
    }
}

#[tokio::test]
async fn fsharp_needs_properties_are_filled_by_the_container() {
    if !FsharpNeeds::dotnet_available() {
        return;
    }
    let root = FsharpNeeds::project(FsharpNeeds::CONTROLLER);
    let request = HandlerRequest::route("fsharp_needs", "/stock", HttpMethod::Get);
    let body = FsharpNeeds::invoke(root.path(), FsharpNeeds::ENTRY, &request).await;
    assert_eq!(body, serde_json::json!({ "name": "anvil", "count": 9 }));
}

#[test]
fn an_fsharp_role_taken_twice_is_refused() {
    let root = FsharpNeeds::project(FsharpNeeds::TWICE);
    let refusal = HandlerSource::discover(root.path(), FsharpNeeds::ENTRY)
        .err()
        .map(|failure| failure.to_string())
        .unwrap_or_default();
    assert!(
        refusal.contains("StockController takes StockService twice"),
        "{refusal}"
    );
    assert!(refusal.contains("TY2015"), "{refusal}");
}

#[tokio::test]
async fn an_fsharp_test_unit_fakes_a_needs_property() {
    if !FsharpNeeds::dotnet_available() {
        return;
    }
    let root = FsharpNeeds::project(FsharpNeeds::CONTROLLER);
    let request = HandlerRequest::route("fsharp_needs_unit", "/", HttpMethod::Post);
    let body = FsharpNeeds::invoke(root.path(), "server/tests/StockServiceTest.fs", &request).await;
    let report: tachyon_core::client_tests::TestReport =
        serde_json::from_value(body).expect("report");
    let passed: Vec<(String, bool)> = report
        .cases
        .into_iter()
        .map(|case| (case.name, case.passed))
        .collect();
    assert_eq!(
        passed,
        [
            (String::from("testReadsTheFakeName"), true),
            (String::from("testStartsWithoutAFake"), true),
        ]
    );
}
