import 'models/OrderInput.dart';

@Service
class OrderService {
  String status = 'ready';

  Object? accept(@Check(OrderInput) Object? order) {
    return order;
  }
}
