open Models

[<RequestRoute("/events")>]
[<Public>]
type EventsController(tickets: TicketService) =
    [<StreamRoute>]
    member this.StreamEvents() : seq<Ticket> =
        EventsController.TicketsFor(tickets.levels())

    static member private TicketsFor(levels: int[]) : seq<Ticket> =
        seq {
            for level in levels do
                yield { Sku = "anvil"; Level = level }
        }
