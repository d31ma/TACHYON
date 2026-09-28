open Models

[<RequestRoute("/orders")>]
[<Public>]
type OrdersController(orderService: OrderService) =
    [<GetRoute>]
    member this.GetOrders() : OrderList =
        { Orders = orderService.List() }
