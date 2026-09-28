import 'models/OrderList.dart';

@RequestRoute("/orders")
@Public
class OrdersController {
  @GetRoute
  OrderList getOrders() => OrderList(OrderService.list());
}
