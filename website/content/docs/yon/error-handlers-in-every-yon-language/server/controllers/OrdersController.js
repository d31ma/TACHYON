@RequestRoute("/orders")
@Public
class OrdersController {
  @GetRoute
  getOrder() {
    throw new OrderNotFound('order 42')
  }
}
