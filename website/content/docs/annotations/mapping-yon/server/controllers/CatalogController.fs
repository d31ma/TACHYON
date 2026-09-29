open System.Text.Json.Nodes
open Models

[<RequestRoute("/catalog")>]
[<Public>]
type CatalogController() =
    [<GetRoute>]
    member this.GetCatalog() : CatalogSummary =
        use signal = new BehaviorSignal()
        use context = Behaviors.create(typeof<CatalogSetup>, "public-catalog", signal)
        let mapper = ProductConverter(context)
        let product = BehaviorJSON.Parse("{\"detail\":{\"id\":7},\"name\":\"Anvil\",\"extra\":\"dropped\"}")
        let summary = mapper.toSummary(product)
        { Summary = JsonNode.Parse(summary.ToString()) }
