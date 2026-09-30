open System.Collections.Generic

[<Service>]
type StockService() =
    member val Products : obj = List<Dictionary<string, string>>() with get, set

    member val Restocking : bool = true with get, set
