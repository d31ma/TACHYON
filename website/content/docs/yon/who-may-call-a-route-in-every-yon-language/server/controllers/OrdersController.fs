[<RequestRoute("/orders")>]
[<SignedIn>]
type OrdersController() =
    [<GetRoute>]
    [<Public>]
    member this.GetOrders() : string list =
        [ "lamp"; "desk" ]

    [<PostRoute>]
    member this.PostOrder() : string list =
        [ "chair" ]

    [<DeleteRoute>]
    [<Allow(typeof<Admin>)>]
    member this.DeleteOrders() : string list =
        []
