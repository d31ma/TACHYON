@RequestRoute("/orders")
@SignedIn
class OrdersController {
  @GetRoute
  @Public
  List<String> getOrders() {
    return ['lamp', 'desk'];
  }

  @PostRoute
  List<String> postOrder() {
    return ['chair'];
  }

  @DeleteRoute
  @Allow(Admin)
  List<String> deleteOrders() {
    return [];
  }
}
