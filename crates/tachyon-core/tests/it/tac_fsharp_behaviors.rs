#![allow(clippy::expect_used)]

use std::path::Path;

struct FsharpTacCatalog;

impl FsharpTacCatalog {
    const UNIT: &'static str = "client/tests/CatalogServiceTest.fs";

    const FILES: [(&'static str, &'static str); 8] = [
        (
            "client/setups/CatalogSetup.fs",
            r#"open System
open System.Collections.Generic
open System.Threading.Tasks

[<Setup>]
type CatalogSetup() =
    static member val commands = List<string>() with get
    static member val sequence = 0 with get, set

    static member configure(execution: BehaviorExecution) : BehaviorAdapters =
        let adapters = BehaviorAdapters()
        let values = Dictionary<string, BehaviorJSON>()
        adapters.caches["catalog"] <-
            { new BehaviorCacheAdapter with
                member this.get(key: string, execution: BehaviorExecution) : Task<BehaviorJSON> =
                    match values.TryGetValue(key) with
                    | true, value -> Task.FromResult(value)
                    | _ -> Task.FromResult(BehaviorJSON.Parse("{\"hit\":false}"))

                member this.set(key: string, value: BehaviorJSON, policy: BehaviorJSON, expiresAt: int64, execution: BehaviorExecution) : Task =
                    values[key] <- BehaviorJSON.Object(dict [ "hit", BehaviorJSON.From(true); "value", value; "expiresAt", BehaviorJSON.From(expiresAt) ])
                    Task.CompletedTask

                member this.delete(key: string, execution: BehaviorExecution) : Task =
                    values.Remove(key) |> ignore
                    Task.CompletedTask }
        adapters.entities["catalog"] <-
            { new BehaviorEntityAdapter with
                member this.execute(command: BehaviorJSON, execution: BehaviorExecution) : Task<BehaviorJSON> =
                    Task.FromResult(CatalogSetup.Rows(command)) }
        adapters.entities["ledger"] <-
            { new BehaviorEntityAdapter with
                member this.execute(command: BehaviorJSON, execution: BehaviorExecution) : Task<BehaviorJSON> =
                    Task.FromResult(CatalogSetup.Ledger(command)) }
        adapters.authorize <- Func<_, _, _>(fun call current -> Task.FromResult(true))
        adapters.authorizeEntity <- Func<_, _, _>(fun command current -> Task.FromResult(true))
        adapters

    static member private Rows(command: BehaviorJSON) : BehaviorJSON =
        let rows = BehaviorJSON.Parse("[{\"id\":1,\"status\":\"active\"},{\"id\":2,\"status\":\"retired\"}]")
        let condition = command.Get("query").Get("where").Items[0]
        let keepUnequal = condition.Get("operator").Text = "ne"
        let matches (row: BehaviorJSON) =
            let equal = row.Get(condition.Get("field").Text).ToString() = condition.Get("value").ToString()
            if keepUnequal then not equal else equal
        BehaviorJSON.Array(rows.Items |> Array.filter matches)

    static member private Ledger(command: BehaviorJSON) : BehaviorJSON =
        let transaction = if command.Has("transaction") then command.Get("transaction").Text else "none"
        if command.Get("operation").Text <> "transaction" then
            CatalogSetup.commands.Add("find@" + transaction)
            BehaviorJSON.Parse("[{\"id\":1,\"status\":\"open\"}]")
        elif command.Get("action").Text = "begin" then
            CatalogSetup.sequence <- CatalogSetup.sequence + 1
            let started = "t" + string CatalogSetup.sequence
            CatalogSetup.commands.Add("begin:" + started)
            BehaviorJSON.Object(dict [ "transaction", BehaviorJSON.From(started) ])
        else
            CatalogSetup.commands.Add(command.Get("action").Text + ":" + transaction)
            BehaviorJSON.Parse("{\"ok\":true}")
"#,
        ),
        (
            "client/entities/ProductEntity.fs",
            r#"[<Entity({"adapter":"catalog"})>]
[<Table("products")>]
type ProductEntity() =
    [<Id>]
    member val id = 0 with get, set

    member val status = "" with get, set
"#,
        ),
        (
            "client/entities/EntryEntity.fs",
            r#"[<Entity({"adapter":"ledger"})>]
[<Table("entries")>]
type EntryEntity() =
    [<Id>]
    member val id : int64 = 0L with get, set

    member val status = "" with get, set
"#,
        ),
        (
            "client/stores/ProductStore.fs",
            r#"open System.Threading.Tasks

[<Store(typeof<ProductEntity>, {"fields":["id","status"],"defaultSize":20,"maxSize":100,"setup":"CatalogSetup"})>]
type ProductStore =
    abstract findByStatus: status: BehaviorJSON -> Task<BehaviorJSON>

    [<Query({"operation":"find","where":[{"field":"status","operator":"ne","argument":0}],"orderBy":[{"field":"id","direction":"desc"}]})>]
    abstract excluding: status: BehaviorJSON * [<PageDefaults>] page: BehaviorJSON -> Task<BehaviorJSON>
"#,
        ),
        (
            "client/stores/EntryStore.fs",
            r#"open System.Threading.Tasks

[<Store(typeof<EntryEntity>, {"fields":["id","status"],"defaultSize":20,"maxSize":100,"setup":"CatalogSetup"})>]
type EntryStore =
    abstract findByStatus: status: BehaviorJSON -> Task<BehaviorJSON>
"#,
        ),
        (
            "client/services/CatalogService.fs",
            r#"open System
open System.Threading.Tasks

[<Service>]
[<CacheSettings({"name":"catalog","ttlMs":30000,"maxBytes":65536,"maxEntries":256})>]
type CatalogService(products: ProductStore, entries: EntryStore) =
    let firstWorkingAttempt = 3

    member val behaviors : BehaviorContext = Behaviors.create(typeof<CatalogSetup>, "catalog", new BehaviorSignal()) with get
    member val reads : int = 0 with get, set
    member val attempts : int = 0 with get, set
    member val gate : TaskCompletionSource<bool> = TaskCompletionSource<bool>() with get

    [<Cached>]
    member this.find(sku: BehaviorJSON) : Task<BehaviorJSON> =
        this.reads <- this.reads + 1
        Task.FromResult(CatalogService.Entry(sku, BehaviorJSON.From(this.reads)))

    [<UpdateCache({"method":"find"})>]
    member this.write(sku: BehaviorJSON, reads: BehaviorJSON) : Task<BehaviorJSON> =
        Task.FromResult(CatalogService.Entry(sku, reads))

    [<ClearCache({"method":"find"})>]
    member this.invalidate(sku: BehaviorJSON) : Task<BehaviorJSON> =
        Task.FromResult(CatalogService.Entry(sku, BehaviorJSON.From(this.reads)))

    [<Cached>]
    member this.price([<CacheKey>] sku: BehaviorJSON, note: BehaviorJSON) : Task<BehaviorJSON> =
        this.reads <- this.reads + 1
        Task.FromResult(BehaviorJSON.Object(dict [ "note", note; "reads", BehaviorJSON.From(this.reads) ]))

    [<Retry({"maxRetries":3,"delayMs":1,"multiplier":2,"maxDelayMs":5,"jitterMs":0})>]
    member this.steady() : Task<BehaviorJSON> =
        this.attempts <- this.attempts + 1
        this.FailBeforeTheWorkingAttempt()
        Task.FromResult(BehaviorJSON.From(this.attempts))

    [<Retry({"maxRetries":1,"delayMs":1,"multiplier":1,"maxDelayMs":1,"jitterMs":0})>]
    member this.hopeless() : Task<BehaviorJSON> =
        this.attempts <- this.attempts + 1
        raise (InvalidOperationException("the checkout API is unavailable"))

    [<MaxAtOnce({"limit":1,"policy":"reject"})>]
    member this.crowded() : Task<BehaviorJSON> =
        task {
            let! opened = this.gate.Task
            return BehaviorJSON.From(opened)
        }

    [<AllOrNothing({"adapter":"ledger"})>]
    member this.settle(status: BehaviorJSON) : Task<BehaviorJSON> =
        entries.findByStatus(status)

    [<AllOrNothing({"adapter":"ledger"})>]
    member this.settleThenRefuse(status: BehaviorJSON) : Task<BehaviorJSON> =
        task {
            let! _ = entries.findByStatus(status)
            return raise (BehaviorError("the settlement was refused"))
        }

    member this.active() : Task<BehaviorJSON> =
        products.findByStatus(BehaviorJSON.From("active"))

    member this.others(page: BehaviorJSON) : Task<BehaviorJSON> =
        products.excluding(BehaviorJSON.From("active"), page)

    [<Cached>]
    member this.listed([<PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]})>] page: BehaviorJSON) : Task<BehaviorJSON> =
        Task.FromResult(page)

    member private this.FailBeforeTheWorkingAttempt() : unit =
        if this.attempts < firstWorkingAttempt then
            raise (InvalidOperationException("the checkout API is unavailable"))

    static member private Entry(sku: BehaviorJSON, reads: BehaviorJSON) : BehaviorJSON =
        BehaviorJSON.Object(dict [ "sku", sku; "reads", reads ])
"#,
        ),
        (
            "client/tests/CatalogTestSetup.fs",
            "[<TestSetup>]
type CatalogTestSetup() =
    static member configure(execution: BehaviorExecution) : BehaviorAdapters =
        BehaviorAdapters()
",
        ),
        (
            "client/tests/CatalogServiceTest.fs",
            r#"[<Test(typeof<CatalogTestSetup>, {"partition":"tests"})>]
type CatalogServiceTest() =
    member private this.Catalog() : CatalogService =
        CatalogService(ProductStore(), EntryStore())

    member this.testCachedReadsOnceUntilClearCache() : unit =
        let catalog = this.Catalog()
        let sku = BehaviorJSON.From("anvil")
        Assert.equals("{\"reads\":1,\"sku\":\"anvil\"}", catalog.find(sku).Result.ToString())
        Assert.equals("{\"reads\":1,\"sku\":\"anvil\"}", catalog.find(sku).Result.ToString(), "a second read comes from the cache")
        catalog.write(sku, BehaviorJSON.From(99)).Result |> ignore
        Assert.equals("{\"reads\":99,\"sku\":\"anvil\"}", catalog.find(sku).Result.ToString(), "UpdateCache stores the written value")
        catalog.invalidate(sku).Result |> ignore
        Assert.equals("{\"reads\":2,\"sku\":\"anvil\"}", catalog.find(sku).Result.ToString(), "ClearCache makes the method read again")

    member this.testCacheKeyLeavesOtherParametersOut() : unit =
        let catalog = this.Catalog()
        let first = catalog.price(BehaviorJSON.From("anvil"), BehaviorJSON.From("first")).Result
        let sameKey = catalog.price(BehaviorJSON.From("anvil"), BehaviorJSON.From("second")).Result
        let otherKey = catalog.price(BehaviorJSON.From("lamp"), BehaviorJSON.From("third")).Result
        Assert.equals(first.ToString(), sameKey.ToString(), "a parameter without CacheKey is not part of the key")
        Assert.equals("{\"note\":\"third\",\"reads\":2}", otherKey.ToString(), "a different CacheKey value reads again")

    member this.testRetryRunsUntilAnAttemptWorks() : unit =
        let catalog = this.Catalog()
        Assert.equals("3", catalog.steady().Result.ToString())
        Assert.equals(3, catalog.attempts)

    member this.testRetryGivesUpAfterItsLastAttempt() : unit =
        let catalog = this.Catalog()
        Assert.throws(fun () -> catalog.hopeless()) |> ignore
        Assert.equals(2, catalog.attempts, "maxRetries 1 makes two attempts")

    member this.testMaxAtOnceRefusesACallOverTheLimit() : unit =
        let catalog = this.Catalog()
        let first = catalog.crowded()
        let second =
            try
                catalog.crowded()
            with error ->
                System.Threading.Tasks.Task.FromException<BehaviorJSON>(error)
        catalog.gate.SetResult(true)
        Assert.equals("true", first.Result.ToString(), "the call holding the slot finishes")
        let refused = Assert.throws(fun () -> second)
        Assert.equals("OPERATION_LIMIT", (refused :?> BehaviorError).Code)

    member this.testAllOrNothingCommitsOrRollsBack() : unit =
        CatalogSetup.commands.Clear()
        let catalog = this.Catalog()
        Assert.equals("[{\"id\":1,\"status\":\"open\"}]", catalog.settle(BehaviorJSON.From("open")).Result.ToString())
        Assert.throws(fun () -> catalog.settleThenRefuse(BehaviorJSON.From("open"))) |> ignore
        Assert.equals("begin:t1,find@t1,commit:t1,begin:t2,find@t2,rollback:t2", System.String.Join(",", CatalogSetup.commands))

    member this.testStoreQueriesAndPageDefaults() : unit =
        let catalog = this.Catalog()
        Assert.equals("[{\"id\":1,\"status\":\"active\"}]", catalog.active().Result.ToString(), "a derived query reads through the Store")
        Assert.equals("[{\"id\":2,\"status\":\"retired\"}]", catalog.others(BehaviorJSON.Parse("{\"size\":5}")).Result.ToString(), "a Query method reads through the Store")
        Assert.equals("{\"offset\":0,\"page\":0,\"size\":20,\"sort\":[]}", catalog.listed(BehaviorJSON.Parse("{}")).Result.ToString(), "PageDefaults fills in the default size")
        Assert.equals("{\"offset\":10,\"page\":2,\"size\":5,\"sort\":[\"id:desc\"]}", catalog.listed(BehaviorJSON.Parse("{\"page\":2,\"size\":5,\"sort\":[\"id:desc\"]}")).Result.ToString())
        let tooLarge = Assert.throws(fun () -> catalog.listed(BehaviorJSON.Parse("{\"size\":500}")))
        Assert.equals("PAGE_INVALID", (tooLarge :?> BehaviorError).Code)
"#,
        ),
    ];

    fn dotnet_available() -> bool {
        std::process::Command::new("dotnet")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }

    fn staged() -> tempfile::TempDir {
        let project = tempfile::tempdir().expect("project");
        for (relative, source) in Self::FILES {
            let target = project.path().join(relative);
            std::fs::create_dir_all(target.parent().expect("parent")).expect("folders");
            std::fs::write(target, source).expect("source");
        }
        project
    }
}

#[test]
fn fsharp_tac_caching_policies_and_stores_run_in_a_test_unit() {
    if !FsharpTacCatalog::dotnet_available() {
        eprintln!("skipping: dotnet is unavailable");
        return;
    }
    let project = FsharpTacCatalog::staged();
    let report = tachyon_core::client_tests::run(project.path(), Path::new(FsharpTacCatalog::UNIT))
        .unwrap_or_else(|failure| panic!("F# Tac unit: {failure:?}"));
    assert_eq!(report.cases.len(), 7, "{:?}", report.cases);
    let failing: Vec<_> = report.cases.iter().filter(|case| !case.passed).collect();
    assert!(failing.is_empty(), "{failing:?}");
}
