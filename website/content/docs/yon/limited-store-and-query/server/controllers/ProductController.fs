open System.Text.Json.Nodes
open System.Threading.Tasks
open Models

[<RequestRoute("/products")>]
[<Public>]
type ProductController(products: ProductStore) =
    [<GetRoute>]
    member this.GetProduct() : ProductReport = this.BuildReport().Result

    member private this.BuildReport() : Task<ProductReport> =
        task {
            let activeStatus = BehaviorJSON.From("active")
            let! active = products.findByStatus(activeStatus)
            let! activeCount = products.countByStatus(activeStatus)
            let! others = products.excluding(activeStatus, BehaviorJSON.Parse("{\"size\":5}"))
            return
                { Active = JsonNode.Parse(active.ToString())
                  Count = JsonNode.Parse(activeCount.ToString())
                  Others = JsonNode.Parse(others.ToString()) }
        }
