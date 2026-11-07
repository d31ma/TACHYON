@RequestRoute("/orders")
@Public
class OrdersController {
  /** @type {OrderService} */
  @Needs
  orderService;
  /**
   * @param {any} order
   * @returns {any}
   */
  @PostRoute
  postOrders(@FromBody order) {
    return this.orderService.accept(order);
  }
}
