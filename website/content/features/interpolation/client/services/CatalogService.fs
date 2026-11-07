open Models

[<Service>]
type CatalogService() =
    member val Title : string = "Catalog" with get, set

    member val Count : int = 6 with get, set

    member val Product : Product = { Name = "Anvil"; Price = "£42.00" } with get, set
