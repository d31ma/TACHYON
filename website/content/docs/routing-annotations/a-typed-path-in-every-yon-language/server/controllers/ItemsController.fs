open Models

[<RequestRoute("/items")>]
[<Public>]
type ItemsController() =
    [<GetRoute("/{id}")>]
    member this.GetItem([<FromPath>] id: int64) : Item =
        { Id = id }
