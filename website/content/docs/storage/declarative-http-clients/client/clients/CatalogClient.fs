open System.Threading.Tasks

[<HTTPClient>]
type CatalogClient =
    [<GetRequest("/api/products")>]
    abstract All: unit -> Task<BehaviorJSON>
