[<Service>]
type BasketService() =
    member val Count : int = 0 with get, set

    member this.Doubled() : int =
        this.Count * 2

    [<OnStart>]
    member this.Initialize() : unit =
        this.Count <- 6
