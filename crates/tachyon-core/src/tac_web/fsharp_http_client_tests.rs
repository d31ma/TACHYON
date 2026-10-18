#![allow(clippy::expect_used)]

use super::{BuildJobs, WebLanguage, build_all};
use std::path::Path;
use std::process::Command;

struct FsharpHttpClientPage;

impl FsharpHttpClientPage {
    const SERVICE: &str = "client/services/CatalogService.fs";
    const FILES: [(&str, &str); 3] = [
        (
            "client/setups/CatalogSetup.fs",
            "[<Setup>]\ntype CatalogSetup() =\n    static member configure(execution: BehaviorExecution) : BehaviorAdapters =\n        execution.Check()\n        let adapters: BehaviorAdapters = BehaviorAdapters()\n        adapters.http.Add(\"catalog\", new EchoHttpHelper())\n        adapters\n",
        ),
        (
            "client/helpers/EchoHttpHelper.fs",
            "open System.Collections.Generic\nopen System.Threading.Tasks\n\n[<Helper>]\ntype EchoHttpHelper() =\n    interface BehaviorHTTPAdapter with\n        member this.exchange(request: BehaviorJSON, execution: BehaviorExecution) : Task<BehaviorJSON> =\n            execution.Check()\n            let echo = Dictionary<string, BehaviorJSON>()\n            for field in [ \"method\"; \"path\"; \"query\"; \"headers\"; \"body\" ] do\n                if request.Has(field) then echo[field] <- request.Get(field)\n            echo[\"adapter\"] <- BehaviorJSON.From(\"echo\")\n            Task.FromResult(BehaviorJSON.Object(echo))\n",
        ),
        (
            "client/remotes/CatalogClient.fs",
            "open System.Threading.Tasks\n\n[<HTTPClient({\"path\":\"/api\",\"adapter\":\"catalog\",\"setup\":\"CatalogSetup\"})>]\ntype CatalogClient =\n    [<PutRequest(\"/products/{id}\")>]\n    abstract Replace: [<PathVariable(\"id\")>] id: int -> [<RequestBody>] product: BehaviorJSON -> Task<BehaviorJSON>\n\n    [<PatchRequest(\"/products/{id}\")>]\n    abstract Rename: [<PathVariable(\"id\")>] id: int * [<RequestBody>] change: BehaviorJSON * [<RequestHeader(\"X-Tenant\")>] tenant: string -> Task<BehaviorJSON>\n",
        ),
    ];
    const SERVICE_TEXT: &str = "open System.Threading.Tasks\n\n[<Service>]\ntype CatalogService() =\n    [<Needs>]\n    member val CatalogClient : CatalogClient = Unchecked.defaultof<CatalogClient> with get, set\n\n    member this.Replace(id: int, sku: string) : Task<BehaviorJSON> =\n        let product = BehaviorJSON.Object(dict [ \"sku\", BehaviorJSON.From(sku) ])\n        this.CatalogClient.Replace id product\n\n    member this.Rename(id: int, name: string) : Task<BehaviorJSON> =\n        let change = BehaviorJSON.Object(dict [ \"name\", BehaviorJSON.From(name) ])\n        this.CatalogClient.Rename(id, change, \"acme\")\n";

    const DRIVER: &str = "const { default: CatalogService, __tachyon_event_plan__: plan } = await import('./CatalogService.js')\n\
         const { default: ApplicationEvents } = await import('./application-events.mjs')\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: (error) => { throw error } })\n\
         const catalog = new CatalogService({})\n\
         bus.attach(catalog, plan.types[0], { applicationEventPublisher: null })\n\
         await bus.initialize(catalog)\n\
         const replaced = await catalog.replace(7, 'anvil')\n\
         const renamed = await catalog.rename(8, 'Anvil')\n\
         console.log(JSON.stringify({ replaced, renamed }))\n\
         process.exit(0)\n";

    fn write(root: &Path, relative: &str, bytes: &[u8]) -> std::path::PathBuf {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        std::fs::write(&path, bytes).expect("file");
        path
    }

    async fn publish(root: &Path) {
        for (relative, text) in Self::FILES {
            Self::write(root, relative, text.as_bytes());
        }
        let source = Self::write(root, Self::SERVICE, Self::SERVICE_TEXT.as_bytes());
        let services = [(
            String::from("catalog"),
            String::from("CatalogService"),
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
        .unwrap_or_else(|failure| panic!("{failure}"));
        for (name, bytes) in built.files {
            Self::write(root, &name, &bytes);
        }
        Self::write(
            root,
            "application-events.mjs",
            concat!(
                include_str!("../template/application-events.js"),
                "\nexport default TacApplicationEvents\n"
            )
            .as_bytes(),
        );
    }

    fn drive(root: &Path) -> serde_json::Value {
        Self::write(root, "drive.mjs", Self::DRIVER.as_bytes());
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
}

#[tokio::test]
async fn an_fsharp_page_service_calls_a_curried_and_a_tupled_http_client_member() {
    if !super::fsharp_tests::fsharp_web_ready() {
        eprintln!("skipping: the .NET wasm-tools workload or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    FsharpHttpClientPage::publish(root.path()).await;
    let report = FsharpHttpClientPage::drive(root.path());
    let replaced = &report["replaced"];
    assert_eq!(replaced["method"], "PUT", "{report}");
    assert_eq!(replaced["path"], "/api/products/7", "{report}");
    assert_eq!(replaced["body"]["sku"], "anvil", "{report}");
    assert_eq!(replaced["adapter"], "echo", "{report}");
    let renamed = &report["renamed"];
    assert_eq!(renamed["method"], "PATCH", "{report}");
    assert_eq!(renamed["path"], "/api/products/8", "{report}");
    assert_eq!(renamed["body"]["name"], "Anvil", "{report}");
    assert_eq!(renamed["headers"]["X-Tenant"], "acme", "{report}");
    assert_eq!(renamed["adapter"], "echo", "{report}");
}
