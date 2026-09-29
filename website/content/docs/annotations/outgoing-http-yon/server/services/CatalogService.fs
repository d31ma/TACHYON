[<Service>]
type CatalogService(catalog: CatalogClient) =
    let itemsField = "items"

    member this.Products() : obj =
        let answer = catalog.All(BehaviorJSON.From(1)).Result
        let texts = answer.Get(itemsField).Items |> Array.map (fun item -> item.Text)
        dict [ itemsField, box texts ] :> obj
