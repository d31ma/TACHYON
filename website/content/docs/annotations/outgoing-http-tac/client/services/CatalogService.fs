open System.Threading.Tasks

[<Service>]
type CatalogService() =
    let catalog = CatalogClient()
    member val count : int = 0 with get, set

    member this.All() : Task<BehaviorJSON> = catalog.All()
