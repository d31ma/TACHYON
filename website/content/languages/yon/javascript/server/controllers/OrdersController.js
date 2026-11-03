@RequestRoute("/orders")
@Public
class OrdersController {
  /** @returns {OrderList} */
  @GetRoute
  getOrders() {
    return { orders: new OrderService().list() }
  }
}
