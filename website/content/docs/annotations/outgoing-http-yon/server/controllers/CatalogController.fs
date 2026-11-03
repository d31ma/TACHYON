[<RequestRoute("/catalog")>]
[<Public>]
type CatalogController(catalog: CatalogService) =
    [<GetRoute>]
    member this.GetCatalog() : obj = catalog.Products()
