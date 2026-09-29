open System.Threading.Tasks

[<HTTPClient>]
type CatalogClient =
    [<GetRequest("/products")>]
    abstract All: [<RequestParam("page")>] page: BehaviorJSON -> Task<BehaviorJSON>

    [<PostRequest("/products")>]
    abstract Add: [<RequestBody>] product: BehaviorJSON -> Task<BehaviorJSON>
