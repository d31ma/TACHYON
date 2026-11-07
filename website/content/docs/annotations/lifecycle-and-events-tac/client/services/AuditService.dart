import 'models/PlacedOrder.dart';
import 'models/SettledOrder.dart';

@Service
class AuditService {
  List<Object?> _log = [];
  Object? settled = 0;
  Object? _audited = 0;

  @OnEvent({"id":"audit-log","replay":true})
  @Order(-10)
  void record(PlacedOrder order) {
    _log = [..._log, order.status];
  }

  @OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})
  @Order(10)
  SettledOrder settle(PlacedOrder order) {
    return SettledOrder(order.total);
  }

  @OnEvent
  void saveSettled(SettledOrder order) {
    settled = order.total;
  }

  @OnEvent
  @InBackground
  Future<void> audit(SettledOrder order) async {
    _audited = order.total;
  }
}
