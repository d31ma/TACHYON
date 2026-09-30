open Models

[<RequestRoute("/inventory")>]
[<Public>]
type InventoryController(applicationEventPublisher: ApplicationEventPublisher) =
    [<PostRoute>]
    member this.PostRecord([<FromBody>] record: InventoryRecord) : InventoryRecord =
        applicationEventPublisher.PublishEvent(record) |> ignore
        record
