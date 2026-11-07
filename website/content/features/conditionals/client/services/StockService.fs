open Models

[<Service>]
type StockService() =
    member val Products : Product list = [] with get, set

    member val Restocking : bool = true with get, set
