[<Service>]
type TickerService() =
    member val Seconds : int = 0 with get, set

    [<Every("1s")>]
    member this.Count() : unit =
        this.Seconds <- this.Seconds + 1
