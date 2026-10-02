#![allow(clippy::expect_used)]

use std::path::Path;

struct CsharpHelperFixture;

impl CsharpHelperFixture {
    const HELPER: &'static str = r#"[Helper]
public sealed class AnswerHelper : BehaviorHTTPAdapter
{
    private readonly string text;
    public AnswerHelper(string text) { this.text = text; }
    public string Read() { return text; }
    public System.Threading.Tasks.Task<BehaviorJSON> exchange(BehaviorJSON request, BehaviorExecution execution)
    {
        execution.Check();
        return System.Threading.Tasks.Task.FromResult(BehaviorJSON.Parse("{\"message\":\"" + text + "\"}"));
    }
}
"#;
    const FIRST: &'static str = r#"[TestSetup]
public sealed class FirstHelperTestSetup
{
    public static BehaviorAdapters configure(BehaviorExecution execution)
    {
        execution.Check();
        var adapters = new BehaviorAdapters();
        adapters.http["probe"] = new AnswerHelper("first");
        return adapters;
    }
}
"#;
    const SECOND: &'static str = r#"[Setup]
public sealed class SecondHelperSetup
{
    public static BehaviorAdapters configure(BehaviorExecution execution)
    {
        execution.Check();
        var adapters = new BehaviorAdapters();
        adapters.http["probe"] = new AnswerHelper("second");
        return adapters;
    }
}
"#;
    const SERVICE: &'static str = r#"[Service]
public sealed class ManualHelperService
{
    private readonly AnswerHelper helper;
    public ManualHelperService() { helper = new AnswerHelper("service"); }
    public string Read() { return helper.Read(); }
}
"#;
    const UNIT: &'static str = r#"[Test(typeof(FirstHelperTestSetup))]
public sealed class CallerOwnedHelperTest
{
    public async System.Threading.Tasks.Task testContextOwnedAdaptersRemainIndependent()
    {
        using var firstSignal = new BehaviorSignal();
        using var secondSignal = new BehaviorSignal();
        using var first = Behaviors.create(typeof(FirstHelperTestSetup), "first", firstSignal);
        using var second = Behaviors.create(typeof(SecondHelperSetup), "second", secondSignal);
        Assert.equals("first", (await first.restClient("probe").get("/probe").retrieve()).Get("message").Text);
        Assert.equals("second", (await second.restClient("probe").get("/probe").retrieve()).Get("message").Text);
        first.Dispose();
        BehaviorError error = null;
        try { await first.restClient("probe").get("/probe").retrieve(); }
        catch (BehaviorError failure) { error = failure; }
        Assert.isTrue(error != null);
        Assert.equals("CANCELLED", error.Code);
        Assert.equals("second", (await second.restClient("probe").get("/probe").retrieve()).Get("message").Text);
    }

    public void testServiceRetainsItsExplicitHelper()
    {
        var first = new ManualHelperService();
        var second = new ManualHelperService();
        Assert.isTrue(!System.Object.ReferenceEquals(first, second));
        Assert.equals("service", first.Read());
        Assert.equals("service", second.Read());
    }
}
"#;

    fn write(root: &Path, relative: &str, source: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(path, source).expect("source");
    }
}

#[test]
#[ignore = "requires installed .NET; runs genuine caller-owned C# Helpers through native Test assembly"]
fn csharp_tac_helpers_compile_as_explicit_context_and_service_collaborators() {
    let root = tempfile::tempdir().expect("C# Helper project");
    for (relative, source) in [
        (
            "client/helpers/http/AnswerHelper.cs",
            CsharpHelperFixture::HELPER,
        ),
        (
            "client/tests/FirstHelperTestSetup.cs",
            CsharpHelperFixture::FIRST,
        ),
        (
            "client/setups/SecondHelperSetup.cs",
            CsharpHelperFixture::SECOND,
        ),
        (
            "client/services/ManualHelperService.cs",
            CsharpHelperFixture::SERVICE,
        ),
        (
            "client/tests/CallerOwnedHelperTest.cs",
            CsharpHelperFixture::UNIT,
        ),
    ] {
        CsharpHelperFixture::write(root.path(), relative, source);
    }
    let report = super::run(
        root.path(),
        Path::new("client/tests/CallerOwnedHelperTest.cs"),
    )
    .expect("supervised native C# Test");
    assert!(report.failure.is_none(), "{report:?}");
    assert_eq!(report.cases.len(), 2, "{report:?}");
    assert!(report.cases.iter().all(|case| case.passed), "{report:?}");
}
