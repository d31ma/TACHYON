open Models

[<Service>]
type StreamService() =
    let tickLimit : int = 128

    [<Needs>]
    member val TicksClient : TicksClient = Unchecked.defaultof<TicksClient> with get, set

    member val Ticks : Tick list = [] with get, set

    [<OnEvent>]
    member this.AddTick(tick: Tick) : unit =
        let ticks = this.Ticks @ [ tick ]
        this.Ticks <- List.skip (max 0 (ticks.Length - tickLimit)) ticks
