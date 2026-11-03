open Models

[<RequestRoute("/items")>]
[<Public>]
type ItemsController() =
    [<GetRoute>]
    member this.GetItems() : ItemList =
        { Items = [] }

    [<PostRoute>]
    [<ResponseStatus(201)>]
    member this.PostItem([<FromBody>] item: Item) : Item =
        item
