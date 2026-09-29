open System.Text.Json.Nodes
open System.Threading.Tasks
open Models

[<RequestRoute("/products")>]
[<Public>]
type ProductController() =
    [<GetRoute>]
    member this.GetProduct() : ProductReport = this.BuildReport().Result

    member private this.BuildReport() : Task<ProductReport> =
        task {
            use signal = new BehaviorSignal()
            use context = Behaviors.create(typeof<CatalogSetup>, "public-catalog", signal)
            let products = ProductStore(context)
            let activeStatus = BehaviorJSON.From("active")
            let! active = products.findByStatus(activeStatus)
            let! activeCount = products.countByStatus(activeStatus)
            let page = products.bounded(BehaviorJSON.Parse("{\"size\":5}"))
            let! others = products.excluding(activeStatus, page)
            return
                { Active = JsonNode.Parse(active.ToString())
                  Count = JsonNode.Parse(activeCount.ToString())
                  Others = JsonNode.Parse(others.ToString()) }
        }
