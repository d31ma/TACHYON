import 'models/Problem.dart';

@RequestRoute("/orders")
@Public
class OrdersController {
  @GetRoute
  Problem getOrder() {
    throw OrderNotFound('order 42');
  }
}
