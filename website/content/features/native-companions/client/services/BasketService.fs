[<Service>]
type BasketService() =
    member val Count = 6 with get, set

    member this.Label : string = "F#"

    member this.Doubled() : int =
        this.Count * 2
