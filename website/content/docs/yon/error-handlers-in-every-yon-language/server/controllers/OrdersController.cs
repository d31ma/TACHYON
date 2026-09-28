using Models;

[RequestRoute("/orders")]
[Public]
class OrdersController {
    [GetRoute]
    public Problem GetOrder() {
        throw new OrderNotFound("order 42");
    }
}
