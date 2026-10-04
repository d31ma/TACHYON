[<RequestRoute("/events")>]
[<Public>]
type EventsController(eventService: EventService) =
    [<GetRoute>]
    member this.GetEvents() : obj =
        eventService.Report()
