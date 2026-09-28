import 'models/Problem.dart';

@ErrorHandler
class OrdersErrorHandler {
  @HandlesError
  @ResponseStatus(404)
  Problem missing(OrderNotFound error) {
    return const Problem('order-not-found');
  }
}
