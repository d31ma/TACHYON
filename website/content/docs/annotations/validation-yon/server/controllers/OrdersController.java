@RequestRoute("/orders")
@Public
class OrdersController {
    @Needs
    final OrderService orders;
    @PostRoute
    Object postOrders(@FromBody Object order) {
        return orders.accept(order);
    }
}
