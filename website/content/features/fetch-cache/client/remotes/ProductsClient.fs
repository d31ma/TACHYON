open System.Threading.Tasks

[<CacheSettings({"name":"products","ttlMs":60000,"maxBytes":65536,"maxEntries":16})>]
[<HTTPClient>]
type ProductsClient =
    [<Cached>]
    [<GetRequest("/api/products")>]
    abstract List: unit -> Task<BehaviorJSON>

    [<ClearCache({"method":"List"})>]
    [<PostRequest("/api/products")>]
    abstract Save: [<RequestBody>] product: BehaviorJSON -> Task<BehaviorJSON>
