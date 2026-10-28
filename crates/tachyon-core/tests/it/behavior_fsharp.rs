#![allow(clippy::expect_used)]

use tachyon_contracts::{HandlerRequest, HttpMethod};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
};

struct FsharpCatalog;

impl FsharpCatalog {
    fn staged(relative: &str, source: &str) -> (String, String) {
        let relative = relative.replacen(
            "server/helpers/OrderInput.fs",
            "server/models/OrderInput.fs",
            1,
        );
        let source = if relative == "server/services/OrderService.fs" {
            format!("open Models\n\n{source}")
        } else {
            source.to_owned()
        };
        (relative, source)
    }

    pub(super) const FILES: [(&str, &str); 11] = [
        (
            "server/models/CatalogReport.fs",
            "open System.Text.Json.Nodes\n\ntype CatalogReport =\n    { First: JsonNode\n      Repeat: JsonNode\n      Written: JsonNode\n      Refreshed: JsonNode\n      Accepted: JsonNode\n      Rejected: string\n      Summary: JsonNode\n      Active: JsonNode\n      Others: JsonNode\n      Priced: JsonNode\n      Repriced: JsonNode\n      OtherPrice: JsonNode }\n",
        ),
        (
            "server/setups/CatalogSetup.fs",
            "open System\nopen System.Threading.Tasks\n\n[<Setup>]\ntype CatalogSetup() =\n    static member configure(execution: BehaviorExecution) : BehaviorAdapters =\n        let adapters = BehaviorAdapters()\n        adapters.caches[\"catalog\"] <- MemoryCacheDelegate()\n        adapters.entities[\"catalog\"] <- CatalogAdapterDelegate()\n        adapters.authorize <- Func<_, _, _>(fun call current -> Task.FromResult(true))\n        adapters.authorizeEntity <- Func<_, _, _>(fun command current -> Task.FromResult(true))\n        adapters\n",
        ),
        (
            "server/delegates/MemoryCacheDelegate.fs",
            "open System.Collections.Generic\nopen System.Threading.Tasks\n\n[<Delegate>]\ntype MemoryCacheDelegate() =\n    let values = Dictionary<string, BehaviorJSON>()\n\n    interface BehaviorCacheAdapter with\n        member this.get(key: string, execution: BehaviorExecution) : Task<BehaviorJSON> =\n            match values.TryGetValue(key) with\n            | true, value -> Task.FromResult(value)\n            | _ -> Task.FromResult(BehaviorJSON.Parse(\"{\\\"hit\\\":false}\"))\n\n        member this.set(key: string, value: BehaviorJSON, policy: BehaviorJSON, expiresAt: int64, execution: BehaviorExecution) : Task =\n            values[key] <- BehaviorJSON.Object(dict [ \"hit\", BehaviorJSON.From(true); \"value\", value; \"expiresAt\", BehaviorJSON.From(expiresAt) ])\n            Task.CompletedTask\n\n        member this.delete(key: string, execution: BehaviorExecution) : Task =\n            values.Remove(key) |> ignore\n            Task.CompletedTask\n",
        ),
        (
            "server/delegates/CatalogAdapterDelegate.fs",
            "open System.Threading.Tasks\n\n[<Delegate>]\ntype CatalogAdapterDelegate() =\n    let rows = BehaviorJSON.Parse(\"[{\\\"id\\\":1,\\\"status\\\":\\\"active\\\"},{\\\"id\\\":2,\\\"status\\\":\\\"retired\\\"}]\")\n\n    interface BehaviorEntityAdapter with\n        member this.execute(command: BehaviorJSON, execution: BehaviorExecution) : Task<BehaviorJSON> =\n            let query = command.Get(\"query\")\n            let condition = query.Get(\"where\").Items[0]\n            let keepUnequal = condition.Get(\"operator\").Text = \"ne\"\n            let matches (row: BehaviorJSON) =\n                let equal = row.Get(condition.Get(\"field\").Text).ToString() = condition.Get(\"value\").ToString()\n                if keepUnequal then not equal else equal\n            let found = rows.Items |> Array.filter matches\n            Task.FromResult(BehaviorJSON.Array(found))\n",
        ),
        (
            "server/helpers/OrderInput.fs",
            "[<Checked>]\ntype OrderInput() =\n    [<NotEmpty>]\n    [<Length({\"min\":1,\"max\":80})>]\n    member val name : BehaviorJSON = BehaviorJSON.Parse(\"null\") with get, set\n\n    [<Required>]\n    [<Min(1)>]\n    [<Max(9)>]\n    member val level : BehaviorJSON = BehaviorJSON.Parse(\"null\") with get, set\n",
        ),
        (
            "server/services/OrderService.fs",
            "[<Service>]\ntype OrderService() =\n    member this.accept([<Check(typeof<OrderInput>)>] order: BehaviorJSON) : BehaviorJSON = order\n",
        ),
        (
            "server/services/CatalogService.fs",
            "open System.Threading.Tasks\n\n[<Service>]\n[<CacheSettings({\"name\":\"catalog\",\"ttlMs\":30000,\"maxBytes\":65536,\"maxEntries\":256})>]\ntype CatalogService() =\n    let mutable reads = 0\n    let mutable prices = 0\n\n    member val behaviors : BehaviorContext = Behaviors.create(typeof<CatalogSetup>, \"public-catalog\", new BehaviorSignal()) with get\n\n    [<Cached>]\n    member this.find(sku: BehaviorJSON) : Task<BehaviorJSON> =\n        task {\n            reads <- reads + 1\n            return CatalogService.Entry(sku, BehaviorJSON.From(reads))\n        }\n\n    [<Cached>]\n    member this.price([<CacheKey>] sku: BehaviorJSON, note: BehaviorJSON) : Task<BehaviorJSON> =\n        task {\n            prices <- prices + 1\n            return BehaviorJSON.From(prices)\n        }\n\n    [<UpdateCache({\"method\":\"find\"})>]\n    member this.write(sku: BehaviorJSON, reads: BehaviorJSON) : Task<BehaviorJSON> =\n        Task.FromResult(CatalogService.Entry(sku, reads))\n\n    [<ClearCache({\"method\":\"find\"})>]\n    member this.invalidate(sku: BehaviorJSON) : Task<BehaviorJSON> =\n        Task.FromResult(CatalogService.Entry(sku, BehaviorJSON.From(reads)))\n\n    static member private Entry(sku: BehaviorJSON, reads: BehaviorJSON) : BehaviorJSON =\n        BehaviorJSON.Object(dict [ \"sku\", sku; \"reads\", reads ])\n",
        ),
        (
            "server/converters/ProductConverter.fs",
            "[<Converter>]\ntype ProductConverter =\n    [<Convert({\"fields\":[{\"source\":\"detail.id\",\"target\":\"productId\"},{\"source\":\"name\",\"target\":\"title\"},{\"target\":\"kind\",\"constant\":\"product\"}]})>]\n    abstract toSummary: product: BehaviorJSON -> BehaviorJSON\n",
        ),
        (
            "server/entities/ProductEntity.fs",
            "[<Entity({\"adapter\":\"catalog\"})>]\n[<Table(\"products\")>]\ntype ProductEntity() =\n    [<Id>]\n    member val id = 0 with get, set\n\n    member val status = \"\" with get, set\n",
        ),
        (
            "server/stores/ProductStore.fs",
            "open System.Threading.Tasks\n\n[<Store(typeof<ProductEntity>, {\"fields\":[\"id\",\"status\"],\"defaultSize\":20,\"maxSize\":100,\"setup\":\"CatalogSetup\"})>]\ntype ProductStore =\n    abstract findByStatus: status: BehaviorJSON -> Task<BehaviorJSON>\n\n    [<Query({\"operation\":\"find\",\"where\":[{\"field\":\"status\",\"operator\":\"ne\",\"argument\":0}],\"orderBy\":[{\"field\":\"id\",\"direction\":\"desc\"}]})>]\n    abstract excluding: status: BehaviorJSON * [<PageDefaults>] page: BehaviorJSON -> Task<BehaviorJSON>\n",
        ),
        (
            "server/controllers/CatalogController.fs",
            "open System.Text.Json.Nodes\nopen Models\n\n[<RequestRoute(\"/catalog\")>]\n[<Public>]\ntype CatalogController(catalog: CatalogService, orders: OrderService, products: ProductStore, converter: ProductConverter) =\n    [<GetRoute>]\n    member this.GetCatalog() : CatalogReport =\n        let sku = BehaviorJSON.From(\"anvil\")\n        let first = catalog.find(sku).Result\n        let repeat = catalog.find(sku).Result\n        catalog.write(sku, BehaviorJSON.From(99)).Result |> ignore\n        let written = catalog.find(sku).Result\n        catalog.invalidate(sku).Result |> ignore\n        let refreshed = catalog.find(sku).Result\n        let accepted = orders.accept(BehaviorJSON.Parse(\"{\\\"name\\\":\\\"lamp\\\",\\\"level\\\":3}\"))\n        let rejected =\n            try\n                orders.accept(BehaviorJSON.Parse(\"{\\\"name\\\":\\\"lamp\\\",\\\"level\\\":12}\")) |> ignore\n                \"accepted\"\n            with :? BehaviorError as error -> error.Code\n        let summary = converter.toSummary(BehaviorJSON.Parse(\"{\\\"detail\\\":{\\\"id\\\":7},\\\"name\\\":\\\"Anvil\\\"}\"))\n        let active = products.findByStatus(BehaviorJSON.From(\"active\")).Result\n        let others = products.excluding(BehaviorJSON.From(\"active\"), BehaviorJSON.Parse(\"{\\\"size\\\":5}\")).Result\n        let priced = catalog.price(sku, BehaviorJSON.From(\"first\")).Result\n        let repriced = catalog.price(sku, BehaviorJSON.From(\"second\")).Result\n        let otherPrice = catalog.price(BehaviorJSON.From(\"bolt\"), BehaviorJSON.From(\"first\")).Result\n        let node (value: BehaviorJSON) = JsonNode.Parse(value.ToString())\n        { First = node first\n          Repeat = node repeat\n          Written = node written\n          Refreshed = node refreshed\n          Accepted = node accepted\n          Rejected = rejected\n          Summary = node summary\n          Active = node active\n          Others = node others\n          Priced = node priced\n          Repriced = node repriced\n          OtherPrice = node otherPrice }\n",
        ),
    ];
}

#[tokio::test]
async fn fsharp_behavior_annotations_run_through_the_csharp_runtime() {
    if std::process::Command::new("dotnet")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let project = tempfile::tempdir().expect("project");
    for (relative, source) in FsharpCatalog::FILES {
        let (relative, source) = FsharpCatalog::staged(relative, source);
        let target = project.path().join(relative);
        std::fs::create_dir_all(target.parent().expect("parent")).expect("folders");
        std::fs::write(target, source).expect("source");
    }
    let relative = "server/controllers/CatalogController.fs";
    let source = HandlerSource::discover(project.path(), relative)
        .unwrap_or_else(|failure| panic!("F# discovery: {failure}"));
    let supervisor =
        HandlerSupervisor::new(HandlerSupervisorOptions::default()).expect("supervisor");
    let response = supervisor
        .invoke(
            &source,
            &HandlerRequest::route("fsharp_behaviors", "/catalog", HttpMethod::Get),
            &HandlerCancellation::default(),
        )
        .await
        .expect("F# invocation");
    assert!(response.error.is_none(), "{:?}", response.error);
    let body: serde_json::Value =
        serde_json::from_str(&response.body.expect("body").data).expect("JSON");
    assert_eq!(
        body["first"],
        serde_json::json!({"sku": "anvil", "reads": 1}),
        "{body}"
    );
    assert_eq!(body["repeat"], body["first"], "{body}");
    assert_eq!(body["written"]["reads"], 99, "{body}");
    assert_eq!(body["refreshed"]["reads"], 2, "{body}");
    assert_eq!(body["accepted"]["level"], 3, "{body}");
    assert_ne!(body["rejected"], "accepted", "{body}");
    assert_eq!(
        body["summary"],
        serde_json::json!({"productId": 7, "title": "Anvil", "kind": "product"}),
        "{body}"
    );
    assert_eq!(
        body["active"],
        serde_json::json!([{"id": 1, "status": "active"}]),
        "{body}"
    );
    assert_eq!(
        body["others"],
        serde_json::json!([{"id": 2, "status": "retired"}]),
        "{body}"
    );
    for (field, calls) in [("priced", 1), ("repriced", 1), ("otherPrice", 2)] {
        assert_eq!(body[field], calls, "{field}: {body}");
    }
}
