@RequestRoute("/orders")
@Public
class OrdersController {
  /** @type {OrderService} */
  @Needs
  orderService;
  /**
   * @param {OrderInput} order
   * @returns {OrderInput}
   */
  @PostRoute
  postOrders(@FromBody order) {
    return this.orderService.accept(order);
  }
}
