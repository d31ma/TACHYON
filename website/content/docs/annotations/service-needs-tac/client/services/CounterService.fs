[<Service>]
type CounterService() =
    member val Count : int = 0 with get, set

    member this.Increment() : int =
        this.Count <- this.Count + 1
        this.Count
