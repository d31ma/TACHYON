[<RequestRoute("/orders")>]
[<Allow(typeof<Admin>)>]
type OrdersController(orders: OrderService) =
    [<GetRoute>]
    member this.GetOrders() : string list =
        failwith "the order store is down"

    [<PostRoute>]
    member this.PostOrder([<FromBody>] order: BehaviorJSON) : BehaviorJSON =
        orders.accept(order)
