import java.util.List;

@RequestRoute("/orders")
@SignedIn
class OrdersController {
    @GetRoute
    @Public
    List<String> getOrders() {
        return List.of("lamp", "desk");
    }

    @PostRoute
    List<String> postOrder() {
        return List.of("chair");
    }

    @DeleteRoute
    @Allow(Admin.class)
    List<String> deleteOrders() {
        return List.of();
    }
}
