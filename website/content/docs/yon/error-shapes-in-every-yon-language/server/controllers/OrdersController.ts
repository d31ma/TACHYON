@RequestRoute("/orders")
@Allow(Admin)
export class OrdersController {
  @Needs
  readonly orders!: OrderService;

  @GetRoute
  getOrders(): string[] {
    throw new RangeError('the order store is down');
  }

  @PostRoute
  postOrder(@FromBody order: any) {
    return this.orders.accept(order);
  }
}
