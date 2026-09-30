open System.Text.Json.Nodes
open Models

[<RequestRoute("/catalog")>]
[<Public>]
type CatalogController(converter: ProductConverter) =
    [<GetRoute>]
    member this.GetCatalog() : CatalogSummary =
        let product = BehaviorJSON.Parse("{\"detail\":{\"id\":7},\"name\":\"Anvil\",\"extra\":\"dropped\"}")
        let summary = converter.toSummary(product)
        { Summary = JsonNode.Parse(summary.ToString()) }
