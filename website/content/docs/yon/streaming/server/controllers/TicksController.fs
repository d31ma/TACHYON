open Models

[<RequestRoute("/ticks")>]
[<Public>]
type TicksController() =
    [<StreamRoute>]
    member this.StreamTicks() : seq<Tick> =
        TicksController.TicksUpTo(3)

    static member private TicksUpTo(last: int) : seq<Tick> =
        seq {
            for level in 1 .. last do
                yield { Sku = "ANVIL-1"; Level = level }
        }
