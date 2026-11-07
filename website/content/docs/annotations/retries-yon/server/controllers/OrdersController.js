/** @import { OrderReport } from '/models/OrderReport.ts' */
@RequestRoute("/orders")
@Public
class OrdersController {
  /** @type {OrdersService} */
  @Needs
  ordersService;

  /** @returns {Promise<OrderReport>} */
  @GetRoute
  async getOrders() {
    const id = 'A-1';
    const submitted = await this.ordersService.submit(id);
    const report = await this.ordersService.report(id);
    return { submitted, report };
  }
}
