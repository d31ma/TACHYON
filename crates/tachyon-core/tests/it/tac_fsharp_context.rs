#![allow(clippy::expect_used)]

use tachyon_core::client_tests::TestReport;

struct FsharpContext;

impl FsharpContext {
    const UNIT: &str = "client/tests/CatalogServiceTest.fs";

    const FILES: [(&str, &str); 6] = [
        (
            "client/tests/ContextTestSetup.fs",
            "[<TestSetup>]\n[<WhenProfile(\"test\")>]\n[<WhenSetting({\"name\":\"TAC_CATALOG_MODE\",\"havingValue\":\"fast\",\"matchIfMissing\":false})>]\ntype ContextTestSetup() =\n    static member val Configured : int = 0 with get, set\n\n    static member configure(execution: BehaviorExecution) : BehaviorAdapters =\n        ContextTestSetup.Configured <- ContextTestSetup.Configured + 1\n        let adapters = BehaviorAdapters()\n        adapters.http.Add(\"prices\", new PriceFakeDelegate(\"from the test setup\"))\n        adapters\n",
        ),
        (
            "client/delegates/PriceFakeDelegate.fs",
            "open System.Threading.Tasks\n\n[<Delegate>]\ntype PriceFakeDelegate(label: string) =\n    interface BehaviorHTTPAdapter with\n        member this.exchange(request: BehaviorJSON, execution: BehaviorExecution) : Task<BehaviorJSON> =\n            execution.Check()\n            Task.FromResult(BehaviorJSON.Object(dict [ \"label\", BehaviorJSON.From(label) ]))\n",
        ),
        (
            "client/setups/CatalogSetup.fs",
            "[<Setup>]\n[<Settings({\"prefix\":\"TAC_CATALOG_\"})>]\n[<WhenProfile(\"public\")>]\n[<WhenSetting({\"name\":\"TAC_CATALOG_CACHE_ENABLED\",\"havingValue\":\"true\",\"matchIfMissing\":false})>]\ntype CatalogSetup() =\n    [<Min(1)>]\n    [<Max(100)>]\n    member val pageSize : int = 20 with get, set\n\n    member val title : string = \"Catalog\" with get, set\n\n    static member val Configured : int = 0 with get, set\n\n    static member configure(execution: BehaviorExecution) : BehaviorAdapters =\n        CatalogSetup.Configured <- CatalogSetup.Configured + 1\n        let adapters = BehaviorAdapters()\n        adapters.http.Add(\"prices\", new PriceFakeDelegate(\"from the catalog setup\"))\n        adapters\n",
        ),
        (
            "client/models/CatalogValues.fs",
            "type CatalogValues() =\n    [<Setting(\"${tac.catalog.title:Untitled}\")>]\n    member val title : string = \"\" with get, set\n\n    [<Setting({\"path\":\"tac.catalog.page.size\",\"type\":\"integer\",\"default\":5,\"min\":1,\"max\":100})>]\n    member val size : int64 = 0L with get, set\n",
        ),
        (
            "client/remotes/PriceClient.fs",
            "open System.Threading.Tasks\n\n[<HTTPClient({\"path\":\"/api/prices\",\"adapter\":\"prices\",\"setup\":\"CatalogSetup\"})>]\ntype PriceClient =\n    [<GetRequest>]\n    abstract read: unit -> Task<BehaviorJSON>\n",
        ),
        (
            "client/services/PriceService.fs",
            "open System.Threading.Tasks\n\n[<Service>]\ntype PriceService() =\n    [<Needs>]\n    member val PriceClient : PriceClient = Unchecked.defaultof<PriceClient> with get, set\n\n    member this.Label() : string =\n        this.PriceClient.read().GetAwaiter().GetResult().Get(\"label\").Text\n",
        ),
    ];

    const MARKERS: &str = "open Models\n\n[<Test(typeof<ContextTestSetup>, {\"partition\":\"tests\"})>]\n[<UseProfiles(\"test\")>]\n[<TestSettings({\"tac.catalog.mode\":\"fast\",\"tac.catalog.size\":5})>]\n[<FreshStart>]\n";

    const CASES: &str = "type CatalogServiceTest() =\n    [<Fake>]\n    member val PriceClient : PriceClient = Unchecked.defaultof<PriceClient> with get, set\n\n    member val Seen : int = 0 with get, set\n\n    member this.testFirstCaseOpensItsOwnSetup() : unit =\n        Assert.equals(0, this.Seen)\n        this.Seen <- 1\n        Assert.equals(1, ContextTestSetup.Configured)\n\n    member this.testSecondCaseStartsFresh() : unit =\n        Assert.equals(0, this.Seen)\n        Assert.equals(2, ContextTestSetup.Configured)\n\n    member this.testBindsSettingsFromApplicationValues() : unit =\n        let bound = Behaviors.properties(typeof<CatalogSetup>, BehaviorJSON.Parse(\"{\\\"tac\\\":{\\\"catalog\\\":{\\\"page\\\":{\\\"size\\\":35},\\\"title\\\":\\\"Shelf\\\"}}}\"))\n        Assert.equals(35L, bound.Get(\"pageSize\").Integer)\n        Assert.equals(\"Shelf\", bound.Get(\"title\").Text)\n        let defaults = Behaviors.properties(typeof<CatalogSetup>, BehaviorJSON.Parse(\"{\\\"tac\\\":{}}\"))\n        Assert.equals(20L, defaults.Get(\"pageSize\").Integer)\n        let refused = Assert.throws(fun () -> Behaviors.properties(typeof<CatalogSetup>, BehaviorJSON.Parse(\"{\\\"tac\\\":{\\\"catalog\\\":{\\\"page\\\":{\\\"size\\\":500}}}}\")) |> ignore)\n        Assert.equals(\"configuration-properties\", (refused :?> BehaviorError).Code)\n\n    member this.testBindsSettingValues() : unit =\n        let values = Behaviors.values(typeof<CatalogValues>, BehaviorJSON.Parse(\"{\\\"tac\\\":{\\\"catalog\\\":{\\\"page\\\":{\\\"size\\\":12}}}}\"))\n        Assert.equals(\"Untitled\", values.Get(\"title\").Text)\n        Assert.equals(12L, values.Get(\"size\").Integer)\n\n    member this.testPicksTheSetupByProfileAndSetting() : unit =\n        use signal = new BehaviorSignal()\n        let enabled = dict [ \"TAC_CATALOG_CACHE_ENABLED\", \"true\" ]\n        let withoutProfile = Assert.throws(fun () -> Behaviors.create(typeof<CatalogSetup>, \"catalog\", signal, 10000, [| \"other\" |], enabled) |> ignore)\n        Assert.equals(\"inactive-configuration\", (withoutProfile :?> BehaviorError).Code)\n        let withoutSetting = Assert.throws(fun () -> Behaviors.create(typeof<CatalogSetup>, \"catalog\", signal, 10000, [| \"public\" |], dict [ \"TAC_CATALOG_CACHE_ENABLED\", \"false\" ]) |> ignore)\n        Assert.equals(\"inactive-configuration\", (withoutSetting :?> BehaviorError).Code)\n        Assert.equals(0, CatalogSetup.Configured)\n        use picked = Behaviors.create(typeof<CatalogSetup>, \"catalog\", signal, 10000, [| \"public\" |], enabled)\n        Assert.equals(1, CatalogSetup.Configured)\n\n    [<ValueSource([1, 50, 100])>]\n    member this.testAcceptsAPageSizeInRange(size: int) : unit =\n        let bound = Behaviors.properties(typeof<CatalogSetup>, dict [ \"TAC_CATALOG_PAGE_SIZE\", string size ])\n        Assert.equals(int64 size, bound.Get(\"pageSize\").Integer)\n\n    member this.testAnEmptyFakeAnswersThroughTheTestSetup() : unit =\n        let prices: PriceService = this.construct<PriceService>()\n        Assert.equals(\"from the test setup\", prices.Label())\n\n    member this.testAnAssignedFakeReplacesTheClient() : unit =\n        use signal = new BehaviorSignal()\n        use catalog = Behaviors.create(typeof<CatalogSetup>, \"catalog\", signal, 10000, [| \"public\" |], dict [ \"TAC_CATALOG_CACHE_ENABLED\", \"true\" ])\n        this.PriceClient <- PriceClient(catalog)\n        let prices: PriceService = this.construct<PriceService>()\n        Assert.equals(\"from the catalog setup\", prices.Label())\n";

    fn report(unit: &str) -> Option<Result<TestReport, String>> {
        let available = std::process::Command::new("dotnet")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success());
        if !available {
            eprintln!("skipping: dotnet is unavailable");
            return None;
        }
        let root = tempfile::tempdir().expect("project");
        for (relative, source) in Self::FILES.iter().chain([&(Self::UNIT, unit)]) {
            let target = root.path().join(relative);
            std::fs::create_dir_all(target.parent().expect("parent")).expect("folders");
            std::fs::write(target, source).expect("source");
        }
        let report = tachyon_core::client_tests::run(root.path(), std::path::Path::new(Self::UNIT))
            .map_err(|failure| format!("{failure:?}"));
        Some(report)
    }

    fn run(unit: &str) -> Option<TestReport> {
        let report = Self::report(unit)?.unwrap_or_else(|failure| panic!("F# unit: {failure}"));
        assert_eq!(report.unit, "CatalogServiceTest");
        Some(report)
    }

    fn refusal(unit: &str) -> Option<String> {
        let report = Self::report(unit)?;
        Some(report.expect_err("the TestSetup refuses the unit"))
    }
}

#[test]
fn a_tac_fsharp_unit_runs_with_its_profiles_settings_inputs_and_fakes() {
    let unit = format!("{}{}", FsharpContext::MARKERS, FsharpContext::CASES);
    let Some(report) = FsharpContext::run(&unit) else {
        return;
    };
    let names: Vec<&str> = report.cases.iter().map(|case| case.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "testFirstCaseOpensItsOwnSetup",
            "testSecondCaseStartsFresh",
            "testBindsSettingsFromApplicationValues",
            "testBindsSettingValues",
            "testPicksTheSetupByProfileAndSetting",
            "testAcceptsAPageSizeInRange[1]",
            "testAcceptsAPageSizeInRange[2]",
            "testAcceptsAPageSizeInRange[3]",
            "testAnEmptyFakeAnswersThroughTheTestSetup",
            "testAnAssignedFakeReplacesTheClient",
        ]
    );
    assert!(
        report.cases.iter().all(|case| case.passed),
        "{:?}",
        report.cases
    );
}

#[test]
fn a_tac_fsharp_unit_without_its_profile_or_setting_is_refused_by_its_test_setup() {
    for markers in [
        "open Models\n\n[<Test(typeof<ContextTestSetup>, {\"partition\":\"tests\"})>]\n[<TestSettings({\"tac.catalog.mode\":\"fast\"})>]\n",
        "open Models\n\n[<Test(typeof<ContextTestSetup>, {\"partition\":\"tests\"})>]\n[<UseProfiles(\"test\")>]\n[<TestSettings({\"tac.catalog.mode\":\"slow\"})>]\n",
    ] {
        let unit = format!("{markers}{}", FsharpContext::CASES);
        let Some(refusal) = FsharpContext::refusal(&unit) else {
            return;
        };
        assert!(
            refusal.contains("inactive-configuration"),
            "{markers}: {refusal}"
        );
    }
}
