open System.Threading.Tasks

[<HTTPClient>]
type CatalogClient() =
    member val behaviors : BehaviorContext = Behaviors.create(typeof<CatalogSetup>, "catalog", new BehaviorSignal())

    [<GetRequest("/products")>]
    abstract All: [<RequestParam("page")>] page: BehaviorJSON -> Task<BehaviorJSON>

    [<PostRequest("/products")>]
    abstract Add: [<RequestBody>] product: BehaviorJSON -> Task<BehaviorJSON>

    [<GetRequest("/products")>]
    member this.First() : Task<BehaviorJSON> =
        Task.FromResult(BehaviorJSON.Parse("{\"query\":{\"page\":1}}"))
