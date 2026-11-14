use super::locks_tests::LocksTests;
use super::{WebApis, WebLanguage, dotnet_wasm_ready};

struct LocksCsharpTests;

impl LocksCsharpTests {
    const CSHARP: &str = r#"[Service]
public class StampService
{
    private readonly Locks locks = new();
    public string Held = "";

    public void Take() => locks.Request("outbox");

    public void Give() => locks.Release("outbox");

    [OnEvent]
    public void HearGranted(LockGranted granted)
    {
        Held = granted.Name;
    }
}
"#;

    const FSHARP: &str = r#"[<Service>]
type StampService() =
    let locks = Locks()
    member val Held: string = "" with get, set
    member this.Take() : unit = locks.Request("outbox")
    member this.Give() : unit = locks.Release("outbox")
    [<OnEvent>]
    member this.HearGranted(granted: LockGranted) : unit = this.Held <- granted.Name
"#;

    async fn assert_service_holds(source: (&str, &str)) {
        if !dotnet_wasm_ready() {
            eprintln!("skipping: node or the .NET wasm-tools workload is unavailable");
            return;
        }
        let root = tempfile::tempdir().expect("project");
        let profile = crate::BuildProfile::Development;
        WebApis::publish_with(root.path(), WebLanguage::CSharp, source, profile).await;
        LocksTests::assert_held(root.path(), false);
    }
}

#[tokio::test]
async fn a_csharp_web_service_takes_and_releases_a_lock() {
    LocksCsharpTests::assert_service_holds(("cs", LocksCsharpTests::CSHARP)).await;
}

#[tokio::test]
async fn an_fsharp_web_service_takes_and_releases_a_lock() {
    LocksCsharpTests::assert_service_holds(("fs", LocksCsharpTests::FSHARP)).await;
}
