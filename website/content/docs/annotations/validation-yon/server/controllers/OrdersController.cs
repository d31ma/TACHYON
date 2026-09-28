[RequestRoute("/orders")]
[Public]
class OrdersController {
    [Needs]
    readonly OrderService _orders;
    [PostRoute]
    public BehaviorJSON PostOrders([FromBody] BehaviorJSON order) => _orders.accept(order);
}
