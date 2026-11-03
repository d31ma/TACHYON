open Models

[<Service>]
type LiveTotalService() =
    member val Seen : int64 = 0L with get, set

    [<OnEvent({"replay":true})>]
    member this.Changed(changed: CartTotal) : unit =
        this.Seen <- changed.Total
