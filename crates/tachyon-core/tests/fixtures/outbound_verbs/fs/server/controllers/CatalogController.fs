[<RequestRoute("/catalog")>]
[<Public>]
type CatalogController(catalog: CatalogClient) =
    [<GetRoute>]
    member this.GetCatalog() : obj =
        dict [
            "found", catalog.Find("anvil", "full", "acme").Result
            "saved", catalog.Save(BehaviorJSON.Parse("{\"sku\":\"anvil\",\"level\":3}")).Result
            "replaced", catalog.Replace(5L, BehaviorJSON.Parse("{\"sku\":\"anvil\",\"level\":4}")).Result
            "renamed", catalog.Rename(6L, BehaviorJSON.Parse("{\"sku\":\"hammer\",\"level\":1}")).Result
            "removed", catalog.Remove(7L).Result
        ] :> obj
