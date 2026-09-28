@RequestRoute("/orders")
@Public
class OrdersController {
  @Needs
  readonly orders!: OrderService;
  @PostRoute
  postOrders(@FromBody order: any) {
    return this.orders.accept(order);
  }
}
