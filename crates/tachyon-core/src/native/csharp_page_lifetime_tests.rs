#![allow(clippy::expect_used)]

use super::registry::NativeCompanionInput;
use crate::project::NativeCompanion;
use std::path::Path;
use std::process::Command;

struct CsharpPageLifetimeCheck;

impl CsharpPageLifetimeCheck {
    const COUNTER: &'static str = r#"[Service]
public class CounterService
{
    [Needs]
    public readonly FilePicker Picker;
    public int Count = 0;
    public int Identity = Probe.Next();
    public void Increment() { Count++; }
    public bool Available() { return Picker.Available(); }
    [OnStart]
    public void Start() { Probe.Log.Add("counter.start"); }
    [OnStop]
    public void Stop() { Probe.Log.Add("counter.stop:" + Picker.Available()); }
}
"#;
    const TALLY: &'static str = r#"[Service]
[Lifetime({"value":"prototype"})]
public class TallyService
{
    public int Count = 0;
    public void Increment() { Count++; }
    [OnStart]
    public void Start() { Probe.Log.Add("tally.start"); }
    [OnStop]
    public void Stop() { Probe.Log.Add("tally.stop"); }
}
"#;
    const LABEL: &'static str = r#"[Service]
[NeedsAll]
[Lifetime({"value":"singleton"})]
public class LabelService
{
    public string Label;
    public string Observed = "";
    [OnStart]
    public void Start() { Observed = Label; }
}
"#;
    const BROKEN: &'static str = r#"[Service]
public class BrokenService
{
    public int Count = 0;
    [OnStart]
    public void Start()
    {
        Probe.Log.Add("broken.start");
        throw new InvalidOperationException("private startup detail");
    }
    [OnStop]
    public void Stop() { Probe.Log.Add("broken.stop"); }
}
"#;
    const CALLER: &'static str = r#"internal static class Probe
{
    internal static readonly List<string> Log = new();
    internal static readonly List<string> Owners = new();
    internal static bool FailConstruction;
    private static int next;
    internal static int Next()
    {
        if (FailConstruction) throw new InvalidOperationException("private constructor detail");
        return ++next;
    }
}

public static class Program
{
    private const int CapabilityOwnerLimit = 256;
    private const string A = "00000000-0000-4000-8000-000000000001";
    private const string B = "00000000-0000-4000-8000-000000000002";
    private const string C = "00000000-0000-4000-8000-000000000003";

    private static string Ask(string route, string service, string owner, string op, string extra = "") =>
        Tac.Invoke("{\"route\":\"" + route + "\",\"service\":\"" + service + "\",\"owner\":\"" + owner + "\",\"op\":\"" + op + "\"" + extra + "}");
    private static string Init(string service, string owner, string route = "/", string props = "") =>
        Ask(route, service, owner, "init", props.Length == 0 ? "" : ",\"props\":" + props);
    private static string Get(string service, string owner, string name, string route = "/") =>
        Ask(route, service, owner, "get", ",\"name\":\"" + name + "\"");
    private static string Call(string service, string owner, string name) =>
        Ask("/", service, owner, "call", ",\"name\":\"" + name + "\",\"args\":[]");
    private static void HostDispose() => Tac.Invoke("{\"op\":\"host-dispose\"}");
    private static void Expect(bool passed, string message)
    {
        if (!passed) throw new Exception(message + " Log: " + string.Join(",", Probe.Log));
    }
    private static void Started(string answer) => Expect(!answer.Contains("error"), "init failed: " + answer);
    private static void Logged(params string[] expected) =>
        Expect(Probe.Log.SequenceEqual(expected), "unexpected lifecycle order, wanted " + string.Join(",", expected) + ".");

    public static void Main()
    {
        NativeCapabilities.Call = (capability, request) =>
        {
            Probe.Owners.Add(JsonNode.Parse(request)["owner"].GetValue<string>());
            return capability == "filePicker.available" ? "{\"status\":\"available\"}" : "{\"status\":\"cancelled\"}";
        };
        SharesOneInstanceUntilPageExit();
        IsolatesExplicitPrototypes();
        StartsOnceAndStopsInReverseCreationOrder();
        RollsBackOnlyWhatAFailedInitCreated();
        AppliesPropertiesBeforeTheOneStart();
        BoundsCapabilityOwnersAndReusesSingletonAtCapacity();
        Expect(Init("counter", "00000000-0000-4000-a000-000000000001").Contains("error"), "a page owner cannot use the reserved provider prefix");
        Console.WriteLine("PASS C# page lifetime");
    }

    private static void SharesOneInstanceUntilPageExit()
    {
        Started(Init("counter", A));
        Started(Init("counter", B));
        Expect(Get("counter", A, "identity") == Get("counter", B, "identity"), "two owners of a page singleton must share one instance");
        Call("counter", A, "increment");
        Expect(Get("counter", B, "count") == "{\"value\":1}", "a page singleton shares its state");
        Logged("counter.start");
        Ask("/", "counter", A, "dispose");
        Expect(Call("counter", B, "available") == "{\"value\":true}", "disposing the first component must not revoke the shared provider capability");
        Expect(Probe.Owners.Count > 0 && Probe.Owners.All(owner => owner.StartsWith("00000000-0000-4000-a000-")), "a page singleton's capabilities belong to the page, not to its first component");
        Ask("/", "counter", B, "dispose");
        Logged("counter.start");
        Started(Init("counter", B));
        Expect(Get("counter", B, "count") == "{\"value\":1}", "page singleton must survive zero temporary component leases");
        Started(Init("counter", C, "/other"));
        Expect(Get("counter", C, "count", "/other") == "{\"value\":0}", "another page gets its own singleton");
        HostDispose();
        Logged("counter.start", "counter.start", "counter.stop:True", "counter.stop:True");
        HostDispose();
        Expect(Probe.Log.Count == 4, "repeated page disposal must not stop twice");
        Started(Init("counter", B));
        Expect(Get("counter", B, "count") == "{\"value\":0}", "page exit must release the singleton");
        HostDispose();
        Probe.Log.Clear();
    }

    private static void IsolatesExplicitPrototypes()
    {
        Started(Init("tally", A));
        Started(Init("tally", B));
        Call("tally", A, "increment");
        Expect(Get("tally", A, "count") == "{\"value\":1}", "a prototype keeps its own state");
        Expect(Get("tally", B, "count") == "{\"value\":0}", "an explicit prototype gives each activation its own instance");
        Ask("/", "tally", A, "dispose");
        Logged("tally.start", "tally.start", "tally.stop");
        Expect(Ask("/", "tally", A, "dispose").Contains("error"), "a disposed prototype owner is stale");
        Logged("tally.start", "tally.start", "tally.stop");
        HostDispose();
        HostDispose();
        Logged("tally.start", "tally.start", "tally.stop", "tally.stop");
        Probe.Log.Clear();
    }

    private static void StartsOnceAndStopsInReverseCreationOrder()
    {
        Started(Init("counter", A));
        Started(Init("tally", B));
        Started(Init("counter", C));
        Logged("counter.start", "tally.start");
        HostDispose();
        Logged("counter.start", "tally.start", "tally.stop", "counter.stop:True");
        Probe.Log.Clear();
    }

    private static void RollsBackOnlyWhatAFailedInitCreated()
    {
        Started(Init("counter", A));
        Call("counter", A, "increment");
        var failed = Init("broken", B);
        Expect(failed.Contains("error") && !failed.Contains("private startup detail"), "a failed start answers a sanitized error: " + failed);
        Logged("counter.start", "broken.start", "broken.stop");
        Expect(Get("counter", A, "count") == "{\"value\":1}", "a failed owner must preserve its preexisting page provider");
        Expect(Init("broken", C).Contains("error"), "a rolled back singleton is constructed again");
        Logged("counter.start", "broken.start", "broken.stop", "broken.start", "broken.stop");
        HostDispose();
        Logged("counter.start", "broken.start", "broken.stop", "broken.start", "broken.stop", "counter.stop:True");
        Probe.Log.Clear();
    }

    private static void AppliesPropertiesBeforeTheOneStart()
    {
        Started(Init("label", A, "/", "{\"label\":\"given\"}"));
        Expect(Get("label", A, "observed") == "{\"value\":\"given\"}", "properties apply before OnStart");
        Started(Init("label", B, "/", "{\"label\":\"second\"}"));
        Expect(Get("label", B, "observed") == "{\"value\":\"given\"}", "page singleton startup must run only once");
        Ask("/", "label", A, "set", ",\"name\":\"label\",\"value\":\"changed\"");
        Expect(Get("label", B, "label") == "{\"value\":\"changed\"}", "existing mutable wire setters remain shared");
        HostDispose();
    }

    private static void BoundsCapabilityOwnersAndReusesSingletonAtCapacity()
    {
        HostDispose();
        Probe.Log.Clear();
        Probe.Owners.Clear();
        for (var index = 0; index < CapabilityOwnerLimit - 1; index++)
        {
            var route = "/capacity/" + index;
            Started(Init("counter", A, route));
            Started(Ask(route, "counter", A, "dispose"));
        }
        Probe.FailConstruction = true;
        string failed;
        try { failed = Init("counter", A, "/capacity/constructor"); }
        finally { Probe.FailConstruction = false; }
        Expect(failed.Contains("error") && !failed.Contains("private constructor detail"), "construction failure must be sanitized");
        Expect(Init("missing", A, "/capacity/missing").Contains("error"), "unknown services must fail without retaining a node");
        Expect(Init("broken", A, "/capacity/startup").Contains("error"), "failed startup must release its node capacity");
        Started(Init("tally", B));
        Started(Init("counter", C, "/capacity/0"));
        Expect(Get("counter", C, "count", "/capacity/0") == "{\"value\":0}", "full capability ownership must let another owner reuse its singleton");
        Expect(Init("counter", A, "/capacity/overflow").Contains("error"), "a new node must respect the native capability owner limit");
        Expect(Probe.Log.Count(entry => entry == "counter.start") == CapabilityOwnerLimit - 1, "a refused node must not run OnStart");
        Started(Ask("/", "tally", B, "dispose"));
        Started(Init("counter", A, "/capacity/overflow"));
        Expect(Init("tally", B).Contains("error"), "prototypes count toward the capability owner limit too");
        HostDispose();
        Expect(Probe.Log.Count(entry => entry == "counter.stop:True") == CapabilityOwnerLimit, "host disposal must release every retained singleton once");
        Expect(Probe.Log.Count(entry => entry == "tally.stop") == 1, "a disposed prototype must release its slot and stop once");
        Started(Init("counter", A, "/after-capacity"));
        HostDispose();
        Probe.Log.Clear();
        Probe.Owners.Clear();
        Console.WriteLine("PASS C# capability owner capacity (native providers intact)");
    }


}
"#;

    fn available() -> bool {
        Command::new("dotnet")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
    }

    fn input(root: &Path, service: &str, class: &str, source: &str) -> NativeCompanionInput {
        let path = root.join(format!("client/services/{class}.cs"));
        std::fs::create_dir_all(path.parent().expect("parent")).expect("services");
        std::fs::write(&path, source).expect("service source");
        NativeCompanionInput {
            snapshot_root: root.to_path_buf(),
            language: NativeCompanion::CSharp,
            source: path,
            service: service.to_owned(),
            class: class.to_owned(),
            web_assembly: false,
            application_provider: false,
        }
    }

    fn registry(root: &Path) -> String {
        let inputs = [
            Self::input(root, "counter", "CounterService", Self::COUNTER),
            Self::input(root, "tally", "TallyService", Self::TALLY),
            Self::input(root, "label", "LabelService", Self::LABEL),
            Self::input(root, "broken", "BrokenService", Self::BROKEN),
        ];
        super::registry::source(&inputs, NativeCompanion::CSharp)
            .expect("registry")
            .expect("source")
    }

    fn run() -> String {
        let root = tempfile::tempdir().expect("project");
        let registry = Self::registry(root.path());
        let program = format!(
            "using System;\nusing System.Collections.Generic;\nusing System.Collections;\nusing System.Globalization;\nusing System.Linq;\nusing System.Text.Json.Nodes;\nusing System.Runtime.InteropServices;\nusing System.Text.Json;\nusing System.Text;\ninternal static class TacApplication {{ internal const string Id = \"test.page-lifetime\"; }}\n{registry}\n{}\n{}\n{}",
            super::windows::companion_prelude(),
            super::csharp_web_apis::CsharpWebApis::native(),
            Self::CALLER
        );
        std::fs::write(root.path().join("Program.cs"), program).expect("program");
        std::fs::write(root.path().join("lifetime.csproj"), "<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup><OutputType>Exe</OutputType><TargetFramework>net9.0</TargetFramework><AllowUnsafeBlocks>true</AllowUnsafeBlocks><Nullable>disable</Nullable><EnableDefaultCompileItems>false</EnableDefaultCompileItems></PropertyGroup><ItemGroup><Compile Include=\"Program.cs\" /></ItemGroup></Project>").expect("project");
        let output = Command::new("dotnet")
            .arg("run")
            .arg("--project")
            .arg(root.path().join("lifetime.csproj"))
            .output()
            .expect("dotnet");
        let printed = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.status.success(), "{printed}");
        printed
    }

    fn namespace(registry: &str, index: usize) -> &str {
        registry
            .split(&format!("namespace TachyonService{index} {{"))
            .nth(1)
            .and_then(|rest| rest.split("namespace TachyonService").next())
            .expect("service namespace")
    }
}

#[test]
fn csharp_registry_declares_each_service_lifetime() {
    let root = tempfile::tempdir().expect("project");
    let registry = CsharpPageLifetimeCheck::registry(root.path());
    let declared = [false, true, false, false];
    for (index, prototype) in declared.into_iter().enumerate() {
        let namespace = CsharpPageLifetimeCheck::namespace(&registry, index);
        assert!(
            namespace.contains(&format!(
                "internal static class TacServiceLifetime {{ internal const bool Prototype = {prototype}; }}"
            )),
            "{namespace}"
        );
    }
    assert!(
        registry.contains("\"tally\" => TachyonService1.TacServiceLifetime.Prototype,"),
        "{registry}"
    );
    assert!(registry.contains("internal static bool Prototype(string service) => service switch"));
    let mut web = CsharpPageLifetimeCheck::input(
        root.path(),
        "counter",
        "CounterService",
        CsharpPageLifetimeCheck::COUNTER,
    );
    web.web_assembly = true;
    let web = super::registry::source(&[web], NativeCompanion::CSharp)
        .expect("web registry")
        .expect("web source");
    assert!(
        web.contains("internal const bool Prototype = true;"),
        "WebAssembly keeps one instance for each browser-side activation"
    );
    let needs_all = CsharpPageLifetimeCheck::input(
        root.path(),
        "tally",
        "TallyService",
        &CsharpPageLifetimeCheck::TALLY
            .replace("[Lifetime({\"value\":\"prototype\"})]", "[NeedsAll]"),
    );
    let needs_all = super::registry::source(&[needs_all], NativeCompanion::CSharp)
        .expect("NeedsAll registry")
        .expect("NeedsAll source");
    assert!(
        needs_all.contains("internal const bool Prototype = true;"),
        "NeedsAll alone keeps one instance for each component"
    );
}

#[test]
fn native_csharp_services_are_page_singletons_unless_declared_prototype() {
    if !CsharpPageLifetimeCheck::available() {
        eprintln!("skipping: dotnet is unavailable");
        return;
    }
    let output = CsharpPageLifetimeCheck::run();
    println!("{output}");
    assert!(output.contains("PASS C# page lifetime"));
}
