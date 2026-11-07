[<RequestRoute("/orders")>]
[<Public>]
type OrdersController(orders: OrderService) =
    [<PostRoute>]
    member this.PostOrders([<FromBody>] order: BehaviorJSON) : BehaviorJSON = orders.accept(order)
