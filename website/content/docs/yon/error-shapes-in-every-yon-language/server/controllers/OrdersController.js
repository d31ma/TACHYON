@RequestRoute("/orders")
@Allow(Admin)
export class OrdersController {
  /** @type {OrderService} */
  @Needs
  orderService;

  /** @returns {string[]} */
  @GetRoute
  getOrders() {
    throw new RangeError('the order store is down')
  }

  /**
   * @param {any} order
   * @returns {any}
   */
  @PostRoute
  postOrder(@FromBody order) {
    return this.orderService.accept(order)
  }
}
