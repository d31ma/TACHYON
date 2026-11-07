open System.Threading.Tasks

[<HTTPClient("/api")>]
type CatalogClient =
    [<GetRequest("/products/{id}")>]
    abstract Find: [<PathVariable("id")>] id: string * [<RequestParam("view")>] view: string * [<RequestHeader("X-Tenant")>] tenant: string -> Task<BehaviorJSON>

    [<PostRequest("/products")>]
    abstract Save: [<RequestBody>] product: BehaviorJSON -> Task<BehaviorJSON>

    [<PutRequest("/products/{id}")>]
    abstract Replace: [<PathVariable("id")>] id: int64 * [<RequestBody>] product: BehaviorJSON -> Task<BehaviorJSON>

    [<PatchRequest("/products/{id}")>]
    abstract Rename: [<PathVariable("id")>] id: int64 * [<RequestBody>] product: BehaviorJSON -> Task<BehaviorJSON>

    [<DeleteRequest("/products/{id}")>]
    abstract Remove: [<PathVariable("id")>] id: int64 -> Task<BehaviorJSON>
