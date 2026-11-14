@RequestRoute("/orders")
@Allow(Admin)
class OrdersController {
  @Needs
  final OrderService orders;

  @GetRoute
  List<String> getOrders() {
    throw StateError('the order store is down');
  }

  @PostRoute
  Object? postOrder(@FromBody Object? order) => orders.accept(order);
}
