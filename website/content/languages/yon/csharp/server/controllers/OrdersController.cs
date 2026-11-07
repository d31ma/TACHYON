using Models;

[RequestRoute("/orders")]
[Public]
class OrdersController
{
    [GetRoute]
    public OrderList GetOrders() =>
        new OrderList(OrderService.List());
}
