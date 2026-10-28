#![allow(clippy::expect_used)]

use std::path::Path;

struct CsharpHelperFixture;

impl CsharpHelperFixture {
    const HELPER: &'static str = r#"[Delegate]
public sealed class AnswerDelegate : BehaviorHTTPAdapter
{
    private readonly string text;
    public AnswerDelegate(string text) { this.text = text; }
    public string Read() { return text; }
    public System.Threading.Tasks.Task<BehaviorJSON> exchange(BehaviorJSON request, BehaviorExecution execution)
    {
        execution.Check();
        return System.Threading.Tasks.Task.FromResult(BehaviorJSON.Parse("{\"message\":\"" + text + "\"}"));
    }
}
"#;
    const FIRST: &'static str = r#"[TestSetup]
public sealed class FirstDelegateTestSetup
{
    public static BehaviorAdapters configure(BehaviorExecution execution)
    {
        execution.Check();
        var adapters = new BehaviorAdapters();
        adapters.http["probe"] = new AnswerDelegate("first");
        return adapters;
    }
}
"#;
    const SECOND: &'static str = r#"[Setup]
public sealed class SecondDelegateSetup
{
    public static BehaviorAdapters configure(BehaviorExecution execution)
    {
        execution.Check();
        var adapters = new BehaviorAdapters();
        adapters.http["probe"] = new AnswerDelegate("second");
        return adapters;
    }
}
"#;
    const SERVICE: &'static str = r#"[Service]
public sealed class ManualDelegateService
{
    private readonly AnswerDelegate helper;
    public ManualDelegateService() { helper = new AnswerDelegate("service"); }
    public string Read() { return helper.Read(); }
}
"#;
    const UNIT: &'static str = r#"[Test(typeof(FirstDelegateTestSetup))]
public sealed class CallerOwnedDelegateTest
{
    public async System.Threading.Tasks.Task testContextOwnedAdaptersRemainIndependent()
    {
        using var firstSignal = new BehaviorSignal();
        using var secondSignal = new BehaviorSignal();
        using var first = Behaviors.create(typeof(FirstDelegateTestSetup), "first", firstSignal);
        using var second = Behaviors.create(typeof(SecondDelegateSetup), "second", secondSignal);
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

    public void testServiceRetainsItsExplicitDelegate()
    {
        var first = new ManualDelegateService();
        var second = new ManualDelegateService();
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
#[ignore = "requires installed .NET; runs genuine caller-owned C# Delegates through native Test assembly"]
fn csharp_tac_helpers_compile_as_explicit_context_and_service_collaborators() {
    let root = tempfile::tempdir().expect("C# Delegate project");
    for (relative, source) in [
        (
            "client/delegates/http/AnswerDelegate.cs",
            CsharpHelperFixture::HELPER,
        ),
        (
            "client/tests/FirstDelegateTestSetup.cs",
            CsharpHelperFixture::FIRST,
        ),
        (
            "client/setups/SecondDelegateSetup.cs",
            CsharpHelperFixture::SECOND,
        ),
        (
            "client/services/ManualDelegateService.cs",
            CsharpHelperFixture::SERVICE,
        ),
        (
            "client/tests/CallerOwnedDelegateTest.cs",
            CsharpHelperFixture::UNIT,
        ),
    ] {
        CsharpHelperFixture::write(root.path(), relative, source);
    }
    let report = super::run(
        root.path(),
        Path::new("client/tests/CallerOwnedDelegateTest.cs"),
    )
    .expect("supervised native C# Test");
    assert!(report.failure.is_none(), "{report:?}");
    assert_eq!(report.cases.len(), 2, "{report:?}");
    assert!(report.cases.iter().all(|case| case.passed), "{report:?}");
}
