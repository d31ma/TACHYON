open System.Threading.Tasks

[<Service>]
[<HTTPClient>]
type CatalogService() =
    member val behaviors : BehaviorContext = Behaviors.create(typeof<CatalogSetup>, "catalog", new BehaviorSignal())
    member val count : int = 0 with get, set

    [<GetRequest("/api/products")>]
    member this.All() : Task<BehaviorJSON> = Task.FromResult(BehaviorJSON.Parse("{}"))
