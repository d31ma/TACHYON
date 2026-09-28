@RequestRoute("/orders")
@Public
class OrdersController {
  @Needs
  final OrderService orders;

  @PostRoute
  Object? postOrders(@FromBody Object? order) => orders.accept(order);
}
