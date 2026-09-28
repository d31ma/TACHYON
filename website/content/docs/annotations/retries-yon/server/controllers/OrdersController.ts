import type { OrderReport } from '/models/OrderReport.ts'

@RequestRoute("/orders")
@Public
class OrdersController {
  @Needs
  readonly ordersService!: OrdersService;

  @GetRoute
  async getOrders(): Promise<OrderReport> {
    const id = 'A-1';
    const submitted = await this.ordersService.submit(id);
    const report = await this.ordersService.report(id);
    return { submitted, report };
  }
}
