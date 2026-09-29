open System.Threading.Tasks

[<CacheSettings({"name":"catalog","ttlMs":30000,"maxBytes":65536,"maxEntries":256})>]
type CatalogService() =
    let skuField = "sku"

    member val behaviors : BehaviorContext = Behaviors.create(typeof<CatalogSetup>, "public-catalog", new BehaviorSignal())
    member val sku : string = "anvil" with get, set
    member val reads : int = 0 with get, set

    [<Cached>]
    member this.find(sku: BehaviorJSON) : Task<BehaviorJSON> =
        this.reads <- this.reads + 1
        Task.FromResult(this.Entry(sku, BehaviorJSON.From(this.reads)))

    [<UpdateCache({"method":"find"})>]
    member this.write(sku: BehaviorJSON, reads: BehaviorJSON) : Task<BehaviorJSON> =
        Task.FromResult(this.Entry(sku, reads))

    [<ClearCache({"method":"find"})>]
    member this.invalidate(sku: BehaviorJSON) : Task<BehaviorJSON> =
        Task.FromResult(BehaviorJSON.Object(dict [ skuField, sku ]))

    member private this.Entry(sku: BehaviorJSON, reads: BehaviorJSON) : BehaviorJSON =
        BehaviorJSON.Object(dict [ skuField, sku; "reads", reads ])
