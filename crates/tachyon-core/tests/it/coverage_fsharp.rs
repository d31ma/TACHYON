#![allow(clippy::expect_used)]

use tachyon_contracts::{HandlerRequest, HttpMethod};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
};

struct FsharpCoverage;

impl FsharpCoverage {
    const SERVICE: &str = "[<Service>]\ntype CatalogService() =\n    member this.Total() : int = 2\n\n    member this.Load(name: string) : string =\n        let trimmed = name.Trim()\n        trimmed\n\n    member this.Untested() : int = 3\n";
    const SETUP: &str = "[<TestSetup>]\ntype CatalogTestSetup() =\n    static member configure(execution: BehaviorExecution) : BehaviorAdapters =\n        BehaviorAdapters()\n";
    const UNIT: &str = "[<Test(typeof<CatalogTestSetup>)>]\ntype CatalogServiceTest() =\n    member this.testTotals() : unit =\n        Assert.equals(2, CatalogService().Total())\n\n    member this.testLoads() : unit =\n        Assert.equals(\"x\", CatalogService().Load(\" x \"))\n";
    const EXPECTED: [(&str, &str); 2] = [
        ("testTotals", "CatalogService.Total"),
        ("testLoads", "CatalogService.Load"),
    ];

    fn dotnet_available() -> bool {
        std::process::Command::new("dotnet")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
    }

    fn project(realm: &str) -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        for (folder, name, source) in [
            ("services", "CatalogService", Self::SERVICE),
            ("tests", "CatalogTestSetup", Self::SETUP),
            ("tests", "CatalogServiceTest", Self::UNIT),
        ] {
            let path = root.path().join(format!("{realm}/{folder}/{name}.fs"));
            std::fs::create_dir_all(path.parent().expect("parent")).expect("folders");
            std::fs::write(path, source).expect("source");
        }
        root
    }

    fn assert_called(cases: &[(String, Vec<String>)]) {
        let expected: Vec<(String, Vec<String>)> = Self::EXPECTED
            .iter()
            .map(|(case, key)| ((*case).to_owned(), vec![(*key).to_owned()]))
            .collect();
        assert_eq!(cases, expected);
    }
}

#[tokio::test]
async fn an_fsharp_yon_case_reports_the_members_it_called() {
    if !FsharpCoverage::dotnet_available() {
        return;
    }
    let root = FsharpCoverage::project("server");
    let relative = "server/tests/CatalogServiceTest.fs";
    let source = HandlerSource::discover(root.path(), relative)
        .unwrap_or_else(|failure| panic!("F# discovery: {failure}"));
    let supervisor =
        HandlerSupervisor::new(HandlerSupervisorOptions::default()).expect("supervisor");
    let response = supervisor
        .invoke(
            &source,
            &HandlerRequest::route("test_unit", "/", HttpMethod::Post),
            &HandlerCancellation::default(),
        )
        .await
        .unwrap_or_else(|error| panic!("F# invocation: {error:?}"));
    assert!(response.error.is_none(), "{:?}", response.error);
    let body = response.body.expect("body").data;
    let report: tachyon_core::client_tests::TestReport =
        serde_json::from_str(&body).expect("JSON report");
    let cases: Vec<(String, Vec<String>)> = report
        .cases
        .into_iter()
        .map(|case| (case.name, case.called))
        .collect();
    FsharpCoverage::assert_called(&cases);
}

#[test]
fn an_fsharp_tac_case_reports_the_members_it_called() {
    if !FsharpCoverage::dotnet_available() {
        return;
    }
    let root = FsharpCoverage::project("client");
    let relative = std::path::Path::new("client/tests/CatalogServiceTest.fs");
    let report = tachyon_core::client_tests::run(root.path(), relative)
        .unwrap_or_else(|failure| panic!("F# Tac unit: {failure:?}"));
    let cases: Vec<(String, Vec<String>)> = report
        .cases
        .into_iter()
        .map(|case| (case.name, case.called))
        .collect();
    FsharpCoverage::assert_called(&cases);
}
