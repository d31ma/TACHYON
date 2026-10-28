#![allow(clippy::expect_used)]

use std::path::Path;

struct FsharpHttpProject;

impl FsharpHttpProject {
    const UNIT: &'static str = "client/tests/CatalogServiceTest.fs";

    const CASES: [&'static str; 6] = [
        "testGetSendsThePathVariableQueryAndHeader",
        "testPostSendsTheBody",
        "testCurriedPutSendsThePathVariableAndBody",
        "testPatchSendsTheBodyAndHeader",
        "testDeleteSendsThePathVariableAndQueryWithoutABody",
        "testEveryRequestGoesThroughTheTestSetupAdapter",
    ];

    const FILES: [(&'static str, &'static str); 6] = [
        (
            "client/setups/CatalogSetup.fs",
            "[<Setup>]\ntype CatalogSetup() =\n    static member configure(execution: BehaviorExecution) : BehaviorAdapters =\n        execution.Check()\n        BehaviorAdapters()\n",
        ),
        (
            "client/delegates/EchoHttpDelegate.fs",
            "open System.Collections.Generic\nopen System.Threading.Tasks\n\n[<Delegate>]\ntype EchoHttpDelegate() =\n    interface BehaviorHTTPAdapter with\n        member this.exchange(request: BehaviorJSON, execution: BehaviorExecution) : Task<BehaviorJSON> =\n            execution.Check()\n            let echo = Dictionary<string, BehaviorJSON>()\n            for field in [ \"method\"; \"path\"; \"query\"; \"headers\"; \"body\" ] do\n                if request.Has(field) then echo[field] <- request.Get(field)\n            echo[\"adapter\"] <- BehaviorJSON.From(\"echo\")\n            Task.FromResult(BehaviorJSON.Object(echo))\n",
        ),
        (
            "client/remotes/CatalogClient.fs",
            "open System.Threading.Tasks\n\n[<HTTPClient({\"path\":\"/api\",\"adapter\":\"catalog\"})>]\ntype CatalogClient =\n    [<GetRequest(\"/products/{id}\")>]\n    abstract Find: [<PathVariable(\"id\")>] id: string * [<RequestParam(\"view\")>] view: string * [<RequestHeader(\"X-Tenant\")>] tenant: string -> Task<BehaviorJSON>\n\n    [<PostRequest(\"/products\")>]\n    abstract Save: [<RequestBody>] product: BehaviorJSON -> Task<BehaviorJSON>\n\n    [<PutRequest(\"/products/{id}\")>]\n    abstract Replace: [<PathVariable(\"id\")>] id: int -> [<RequestBody>] product: BehaviorJSON -> Task<BehaviorJSON>\n\n    [<PatchRequest(\"/products/{id}\")>]\n    abstract Rename: [<PathVariable(\"id\")>] id: int * [<RequestBody>] change: BehaviorJSON * [<RequestHeader(\"X-Tenant\")>] tenant: string -> Task<BehaviorJSON>\n\n    [<DeleteRequest(\"/products/{id}\")>]\n    abstract Remove: [<PathVariable(\"id\")>] id: int * [<RequestParam(\"force\")>] force: bool -> Task<BehaviorJSON>\n",
        ),
        (
            "client/services/CatalogService.fs",
            "open System.Threading.Tasks\n\n[<Service>]\ntype CatalogService() =\n    [<Needs>]\n    member val CatalogClient : CatalogClient = Unchecked.defaultof<CatalogClient> with get, set\n\n    member this.Find(id: string) : BehaviorJSON =\n        this.CatalogClient.Find(id, \"full\", \"acme\").GetAwaiter().GetResult()\n\n    member this.Save(product: BehaviorJSON) : BehaviorJSON =\n        this.CatalogClient.Save(product).GetAwaiter().GetResult()\n\n    member this.Replace(id: int, product: BehaviorJSON) : BehaviorJSON =\n        (this.CatalogClient.Replace id product).GetAwaiter().GetResult()\n\n    member this.Rename(id: int, name: string) : BehaviorJSON =\n        let change = BehaviorJSON.Object(dict [ \"name\", BehaviorJSON.From(name) ])\n        this.CatalogClient.Rename(id, change, \"acme\").GetAwaiter().GetResult()\n\n    member this.Remove(id: int) : BehaviorJSON =\n        this.CatalogClient.Remove(id, true).GetAwaiter().GetResult()\n",
        ),
        (
            "client/tests/CatalogTestSetup.fs",
            "[<TestSetup>]\ntype CatalogTestSetup() =\n    static member configure(execution: BehaviorExecution) : BehaviorAdapters =\n        execution.Check()\n        let adapters: BehaviorAdapters = BehaviorAdapters()\n        adapters.http.Add(\"catalog\", new EchoHttpDelegate())\n        adapters\n",
        ),
        (
            "client/tests/CatalogServiceTest.fs",
            "[<Test(typeof<CatalogTestSetup>, {\"partition\":\"fsharp-http-tests\"})>]\ntype CatalogServiceTest() =\n    [<Fake>]\n    member val CatalogClient : CatalogClient = Unchecked.defaultof<CatalogClient> with get, set\n\n    member private this.Product() : BehaviorJSON =\n        BehaviorJSON.Object(dict [ \"sku\", BehaviorJSON.From(\"anvil\") ])\n\n    member this.testGetSendsThePathVariableQueryAndHeader() : unit =\n        let sent = this.construct<CatalogService>().Find(\"anvil\")\n        Assert.equals(\"GET\", sent.Get(\"method\").Text)\n        Assert.equals(\"/api/products/anvil\", sent.Get(\"path\").Text)\n        Assert.equals(\"full\", sent.Get(\"query\").Get(\"view\").Text)\n        Assert.equals(\"acme\", sent.Get(\"headers\").Get(\"X-Tenant\").Text)\n        Assert.isTrue(not (sent.Has(\"body\")), \"a GET sends no body\")\n\n    member this.testPostSendsTheBody() : unit =\n        let sent = this.construct<CatalogService>().Save(this.Product())\n        Assert.equals(\"POST\", sent.Get(\"method\").Text)\n        Assert.equals(\"/api/products\", sent.Get(\"path\").Text)\n        Assert.equals(\"anvil\", sent.Get(\"body\").Get(\"sku\").Text)\n        Assert.equals(0, sent.Get(\"query\").Members.Count)\n        Assert.equals(0, sent.Get(\"headers\").Members.Count)\n\n    member this.testCurriedPutSendsThePathVariableAndBody() : unit =\n        let sent = this.construct<CatalogService>().Replace(7, this.Product())\n        Assert.equals(\"PUT\", sent.Get(\"method\").Text)\n        Assert.equals(\"/api/products/7\", sent.Get(\"path\").Text)\n        Assert.equals(\"anvil\", sent.Get(\"body\").Get(\"sku\").Text)\n\n    member this.testPatchSendsTheBodyAndHeader() : unit =\n        let sent = this.construct<CatalogService>().Rename(7, \"Anvil\")\n        Assert.equals(\"PATCH\", sent.Get(\"method\").Text)\n        Assert.equals(\"/api/products/7\", sent.Get(\"path\").Text)\n        Assert.equals(\"Anvil\", sent.Get(\"body\").Get(\"name\").Text)\n        Assert.equals(\"acme\", sent.Get(\"headers\").Get(\"X-Tenant\").Text)\n\n    member this.testDeleteSendsThePathVariableAndQueryWithoutABody() : unit =\n        let sent = this.construct<CatalogService>().Remove(7)\n        Assert.equals(\"DELETE\", sent.Get(\"method\").Text)\n        Assert.equals(\"/api/products/7\", sent.Get(\"path\").Text)\n        Assert.equals(\"true\", sent.Get(\"query\").Get(\"force\").ToString())\n        Assert.isTrue(not (sent.Has(\"body\")), \"this DELETE sends no body\")\n\n    member this.testEveryRequestGoesThroughTheTestSetupAdapter() : unit =\n        let catalog = this.construct<CatalogService>()\n        let answers = [ catalog.Find(\"anvil\"); catalog.Save(this.Product()); catalog.Replace(7, this.Product()); catalog.Rename(7, \"Anvil\"); catalog.Remove(7) ]\n        for answer in answers do\n            Assert.equals(\"echo\", answer.Get(\"adapter\").Text)\n",
        ),
    ];

    fn write(root: &Path) {
        for (relative, source) in Self::FILES {
            let path = root.join(relative);
            std::fs::create_dir_all(path.parent().expect("fixture parent"))
                .expect("fixture directory");
            std::fs::write(path, source).expect("fixture source");
        }
    }

    fn dotnet_available() -> bool {
        std::process::Command::new("dotnet")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
    }
}

#[test]
fn an_fsharp_http_client_sends_every_verb_and_request_part_through_its_test_adapter() {
    if !FsharpHttpProject::dotnet_available() {
        eprintln!("skipping: dotnet is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("test project");
    FsharpHttpProject::write(root.path());
    let report = tachyon_core::client_tests::run(root.path(), Path::new(FsharpHttpProject::UNIT))
        .unwrap_or_else(|error| panic!("F# test unit: {error:?}"));
    let names: Vec<&str> = report.cases.iter().map(|case| case.name.as_str()).collect();
    assert_eq!(names, FsharpHttpProject::CASES, "{report:?}");
    assert!(report.cases.iter().all(|case| case.passed), "{report:?}");
}
