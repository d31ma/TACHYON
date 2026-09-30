open System.Collections.Generic

[<Service>]
type CatalogService() =
    static member private Product(id: string, name: string, price: string) : obj =
        let product = Dictionary<string, obj>()
        product["id"] <- id
        product["name"] <- name
        product["price"] <- price
        box product

    member this.Products : obj =
        box [| CatalogService.Product("anvil", "Anvil", "£42.00"); CatalogService.Product("rope", "Rope", "£8.00") |]
