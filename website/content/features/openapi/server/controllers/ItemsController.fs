open Models

[<RequestRoute("/items")>]
[<Public>]
type ItemsController() =
    [<GetRoute("/{id}")>]
    member this.GetItem([<FromPath>] id: int64) : Item =
        { Name = "anvil"; Level = id }

    [<PatchRoute("/{id}")>]
    member this.PatchItem([<FromPath>] id: int64, [<FromBody>] item: Item) : Item =
        { item with Level = id }
