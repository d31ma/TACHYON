[<Service>]
type CounterService() =
    member val Count : int = 0 with get, set

    member this.Increment() : unit =
        this.Count <- this.Count + 1
