import models.Problem;

@RequestRoute("/orders")
@Public
class OrdersController {
    @GetRoute
    Problem getOrder() {
        throw new OrderNotFound("order 42");
    }
}
