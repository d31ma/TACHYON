open Models

[<RequestRoute("/orders")>]
[<Public>]
type OrdersController() =
    [<GetRoute>]
    member this.GetOrder() : Problem =
        raise (OrderNotFound("order 42"))
