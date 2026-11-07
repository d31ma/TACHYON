open System.Threading.Tasks

[<Service>]
type CatalogService() =
    let catalog = CatalogClient()
    member val count : int = 0 with get, set

    member this.Load() : Task =
        task {
            let! answer = catalog.All()
            this.count <- answer.Get("items").Items.Length
        }
