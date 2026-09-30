open System.Collections.Generic

[<Service>]
type CatalogService() =
    member val Title : string = "Catalog" with get, set

    member val Count : int = 6 with get, set

    member val Product : obj = Dictionary<string, string>(dict [ "name", "Anvil"; "price", "£42.00" ]) with get, set
