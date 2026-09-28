import 'models/OrderReport.dart';

@RequestRoute("/orders")
@Public
class OrdersController {
  @Needs
  final OrdersService orders;

  @GetRoute
  Future<OrderReport> getOrders() async {
    final id = 'A-1';
    final submitted = await orders.submit(id);
    final report = await orders.report(id);
    return OrderReport(submitted, report);
  }
}
