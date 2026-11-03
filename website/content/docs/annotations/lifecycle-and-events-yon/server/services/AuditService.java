import models.PlacedOrder;
import models.SettledOrder;

@Service
class AuditService {
  final java.util.List<String> log = new java.util.ArrayList<>();
  long settled = 0;

  @OnEvent({"id":"audit-log"})
  @Order(-10)
  void record(PlacedOrder order) { log.add(order.status()); }

  @OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})
  @Order(10)
  SettledOrder settle(PlacedOrder order) { return new SettledOrder(order.total()); }

  @OnEvent
  void saveSettled(SettledOrder order) { settled = order.total(); }
}
