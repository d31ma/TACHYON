open Models
[<Service>]
type CatalogService(productConverter: ProductConverter) =
    member val result : string = "" with get, set

    member this.load() : unit =
        let detail : Detail = { name = "detail"; secret = "detail-private" }
        let product = productConverter.copy({ id = 7; name = "Anvil"; detail = detail; items = [detail]; secret = "product-private" })
        let summary = productConverter.map(product)
        this.result <- sprintf "%s %d: %s, %s, %d" summary.kind summary.id summary.name summary.detail.name summary.items.Length
