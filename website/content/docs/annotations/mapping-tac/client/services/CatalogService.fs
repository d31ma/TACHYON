[<Service>]
type CatalogService(converter: ProductConverter) =
    member val title : string = "Not mapped." with get, set

    member this.load() : string =
        let product = BehaviorJSON.Parse("{\"detail\":{\"id\":7},\"name\":\"Anvil\",\"extra\":\"dropped\"}")
        let summary = converter.toSummary(product)
        this.title <- summary.Get("kind").Text + " " + summary.Get("productId").ToString() + ": " + summary.Get("title").Text
        this.title
