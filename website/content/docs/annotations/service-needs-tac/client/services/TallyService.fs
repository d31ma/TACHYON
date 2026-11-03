[<Service>]
type TallyService() =
    [<Needs>]
    member val Counter : CounterService = Unchecked.defaultof<CounterService> with get, set

    member val Total : int = 0 with get, set

    member this.Bump() : unit =
        this.Total <- this.Counter.Increment()
