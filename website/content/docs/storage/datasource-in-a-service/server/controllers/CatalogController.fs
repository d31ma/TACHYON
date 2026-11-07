[<RequestRoute("/catalog")>]
[<Public>]
type CatalogController(catalogService: CatalogService) =
    [<GetRoute>]
    member this.GetCatalog() : obj =
        catalogService.Find()
