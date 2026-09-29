open Models

[<RequestRoute("/ticks")>]
[<Public>]
type TicksController() =
    [<StreamRoute>]
    member this.StreamTicks() : seq<Tick> =
        TicksController.TicksFor([| "ANVIL-1"; "ANVIL-2" |])

    static member private TicksFor(skus: string[]) : seq<Tick> =
        seq {
            for sku in skus do
                System.Threading.Thread.Sleep(1000)
                yield { Sku = sku; Level = 3 }
        }
