import models.OrderReport;

@RequestRoute("/orders")
@Public
class OrdersController {
    @Needs
    final OrdersService orders;

    @GetRoute
    OrderReport getOrders() {
        String id = "A-1";
        Object submitted = orders.submit(id);
        Object report = orders.report(id);
        return new OrderReport(submitted, report);
    }
}
