#![allow(clippy::expect_used)]

use std::path::Path;

struct DotnetHelperUnits;

impl DotnetHelperUnits {
    const CSHARP: [(&'static str, &'static str); 5] = [
        (
            "client/helpers/TaxRateHelper.cs",
            "[Helper]\npublic sealed class TaxRateHelper {\n    public int Asked = 0;\n    public int Rate() { Asked += 1; return 2; }\n}\n",
        ),
        (
            "client/helpers/TaxHelper.cs",
            "[Helper]\npublic sealed class TaxHelper {\n    [Needs]\n    private readonly TaxRateHelper _rates;\n    public int AddTax(int subtotal) { return subtotal + _rates.Rate(); }\n    public int Asked() { return _rates.Asked; }\n}\n",
        ),
        (
            "client/services/CheckoutService.cs",
            "[Service]\npublic sealed class CheckoutService {\n    [Needs]\n    private readonly TaxHelper _tax;\n    public int Checkout() { return _tax.AddTax(40); }\n    public int Asked() { return _tax.Asked(); }\n}\n",
        ),
        (
            "client/tests/CheckoutTestSetup.cs",
            "[TestSetup]\npublic sealed class CheckoutTestSetup {\n    public static BehaviorAdapters configure(BehaviorExecution execution) { return new BehaviorAdapters(); }\n}\n",
        ),
        (
            "client/tests/CheckoutServiceTest.cs",
            "[Test(typeof(CheckoutTestSetup))]\npublic sealed class CheckoutServiceTest {\n    public void testEachServiceHasItsOwnHelper() {\n        var first = new CheckoutService();\n        var second = new CheckoutService();\n        Assert.equals(42, first.Checkout());\n        Assert.equals(42, first.Checkout());\n        Assert.equals(42, second.Checkout());\n        Assert.equals(2, first.Asked());\n        Assert.equals(1, second.Asked());\n    }\n}\n",
        ),
    ];

    const FSHARP: [(&'static str, &'static str); 5] = [
        (
            "client/helpers/TaxRateHelper.fs",
            "[<Helper>]\ntype TaxRateHelper() =\n    member val Asked = 0 with get, set\n    member this.Rate() : int =\n        this.Asked <- this.Asked + 1\n        2\n",
        ),
        (
            "client/helpers/TaxHelper.fs",
            "[<Helper>]\ntype TaxHelper(rates: TaxRateHelper) =\n    member this.AddTax(subtotal: int) : int = subtotal + rates.Rate()\n    member this.Asked() : int = rates.Asked\n",
        ),
        (
            "client/services/CheckoutService.fs",
            "[<Service>]\ntype CheckoutService() =\n    [<Needs>]\n    member val Tax : TaxHelper = Unchecked.defaultof<TaxHelper> with get, set\n    member this.Checkout() : int = this.Tax.AddTax(40)\n    member this.Asked() : int = this.Tax.Asked()\n",
        ),
        (
            "client/tests/CheckoutTestSetup.fs",
            "[<TestSetup>]\ntype CheckoutTestSetup() =\n    static member configure(execution: BehaviorExecution) : BehaviorAdapters =\n        BehaviorAdapters()\n",
        ),
        (
            "client/tests/CheckoutServiceTest.fs",
            "[<Test(typeof<CheckoutTestSetup>)>]\ntype CheckoutServiceTest() =\n    member this.testEachServiceHasItsOwnHelper() : unit =\n        let first = CheckoutService()\n        let second = CheckoutService()\n        Assert.equals(42, first.Checkout())\n        Assert.equals(42, first.Checkout())\n        Assert.equals(42, second.Checkout())\n        Assert.equals(2, first.Asked())\n        Assert.equals(1, second.Asked())\n",
        ),
    ];

    fn passes(files: &[(&str, &str)]) {
        let root = tempfile::tempdir().expect("Helper project");
        for (relative, source) in files {
            let path = root.path().join(relative);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
            std::fs::write(path, source).expect("source");
        }
        let (unit, _) = files[files.len() - 1];
        let report = super::run(root.path(), Path::new(unit)).expect("supervised native Test");
        assert!(report.failure.is_none(), "{report:?}");
        assert_eq!(report.cases.len(), 1, "{report:?}");
        assert!(report.cases.iter().all(|case| case.passed), "{report:?}");
    }
}

#[test]
#[ignore = "requires installed .NET; runs a C# service whose [Needs] field names a Helper"]
fn a_csharp_test_unit_gives_each_service_its_own_needed_helper() {
    DotnetHelperUnits::passes(&DotnetHelperUnits::CSHARP);
}

#[test]
#[ignore = "requires installed .NET; runs an F# service whose [<Needs>] property names a Helper"]
fn an_fsharp_test_unit_gives_each_service_its_own_needed_helper() {
    DotnetHelperUnits::passes(&DotnetHelperUnits::FSHARP);
}
