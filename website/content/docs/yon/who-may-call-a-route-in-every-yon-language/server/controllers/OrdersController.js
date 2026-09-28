@RequestRoute("/orders")
@SignedIn
export class OrdersController {
  /** @returns {string[]} */
  @GetRoute
  @Public
  getOrders() {
    return ['lamp', 'desk']
  }

  /** @returns {string[]} */
  @PostRoute
  postOrder() {
    return ['chair']
  }

  /** @returns {string[]} */
  @DeleteRoute
  @Allow(Admin)
  deleteOrders() {
    return []
  }
}
