import 'models/PlacedOrder.dart';
import 'models/SettledOrder.dart';

@Service
class AuditService {
  final List<String> log = [];
  int settled = 0;
  int audited = 0;

  @OnEvent({"id":"audit-log"})
  @Order(-10)
  void record(PlacedOrder order) {
    log.add(order.status);
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
    audited = order.total;
  }
}
