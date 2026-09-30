open Models
[<RequestRoute("/catalog")>]
[<Public>]
type CatalogController(converter: ProductConverter) =
    [<GetRoute>]
    member this.GetCatalog() : Summary =
        let detail : Detail = { name = "detail"; secret = "detail-private" }
        converter.map({ id = 7; name = "Anvil"; detail = detail; items = [detail]; secret = "product-private" })
