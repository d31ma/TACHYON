import java.util.List;

@RequestRoute("/orders")
@Allow(Admin.class)
class OrdersController {
    @Needs
    final OrderService orders;

    @GetRoute
    List<String> getOrders() {
        throw new IllegalStateException("the order store is down");
    }

    @PostRoute
    Object postOrder(@FromBody Object order) {
        return orders.accept(order);
    }
}
