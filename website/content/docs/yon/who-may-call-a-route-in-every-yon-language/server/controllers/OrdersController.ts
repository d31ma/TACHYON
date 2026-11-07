@RequestRoute("/orders")
@SignedIn
export class OrdersController {
  @GetRoute
  @Public
  getOrders(): string[] {
    return ['lamp', 'desk'];
  }

  @PostRoute
  postOrder(): string[] {
    return ['chair'];
  }

  @DeleteRoute
  @Allow(Admin)
  deleteOrders(): string[] {
    return [];
  }
}
