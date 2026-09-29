open System.Text.Json.Nodes
open System.Threading.Tasks
open Models

[<RequestRoute("/catalog")>]
[<Public>]
type CatalogController(catalog: CatalogService) =
    [<GetRoute>]
    member this.GetCatalog() : CacheReport = this.BuildReport().Result

    member private this.BuildReport() : Task<CacheReport> =
        task {
            let sku = BehaviorJSON.From("anvil")
            let! first = catalog.find(sku)
            let! repeat = catalog.find(sku)
            let! _ = catalog.write(sku, BehaviorJSON.From(99))
            let! written = catalog.find(sku)
            let! _ = catalog.invalidate(sku)
            let! refreshed = catalog.find(sku)
            return
                { First = JsonNode.Parse(first.ToString())
                  Repeat = JsonNode.Parse(repeat.ToString())
                  Written = JsonNode.Parse(written.ToString())
                  Refreshed = JsonNode.Parse(refreshed.ToString()) }
        }
