open System.Threading.Tasks

[<Service>]
[<CacheSettings({"name":"catalog","ttlMs":30000,"maxBytes":65536,"maxEntries":256})>]
type CatalogService() =
    let mutable reads = 0

    member val behaviors : BehaviorContext = Behaviors.create(typeof<CatalogSetup>, "public-catalog", new BehaviorSignal())

    [<Cached>]
    member this.find(sku: BehaviorJSON) : Task<BehaviorJSON> =
        reads <- reads + 1
        Task.FromResult(CatalogService.Entry(sku, BehaviorJSON.From(reads)))

    [<UpdateCache({"method":"find"})>]
    member this.write(sku: BehaviorJSON, reads: BehaviorJSON) : Task<BehaviorJSON> =
        Task.FromResult(CatalogService.Entry(sku, reads))

    [<ClearCache({"method":"find"})>]
    member this.invalidate(sku: BehaviorJSON) : Task<BehaviorJSON> =
        Task.FromResult(CatalogService.Entry(sku, BehaviorJSON.From(reads)))

    static member private Entry(sku: BehaviorJSON, reads: BehaviorJSON) : BehaviorJSON =
        BehaviorJSON.Object(dict [ "sku", sku; "reads", reads ])
