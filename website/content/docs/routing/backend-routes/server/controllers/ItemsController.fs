open Models

[<RequestRoute("/items")>]
[<Public>]
type ItemsController() =
    [<GetRoute("/{id}")>]
    member this.GetItem([<FromPath>] id: string) : Item =
        { Name = id }

    [<PostRoute>]
    [<ResponseStatus(201)>]
    member this.PostItem([<FromBody>] item: Item) : Item =
        item
