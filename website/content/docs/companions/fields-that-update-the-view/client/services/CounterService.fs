[<Service>]
type CounterService() =
    member val Count = 0 with get, set

    [<SessionState>]
    member val Visits = 0 with get, set

    [<LocalState>]
    member val Theme = "light" with get, set

    member this.Increment() : unit =
        this.Count <- this.Count + 1
