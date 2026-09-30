open System.Threading.Tasks

[<Service>]
type CatalogService() =
    let products = ProductsClient()
    member val Count : int = 0 with get, set

    member this.Load() : Task =
        task {
            let! answer = products.List()
            this.Count <- answer.Get("items").Items.Length
        }

    member this.Add() : Task =
        task {
            let! _ = products.Save(BehaviorJSON.Parse("{\"name\":\"anvil\"}"))
            return ()
        }
