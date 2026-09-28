import models.PlacedOrder
import models.SettledOrder

@Service
class AuditService {
  val log = mutableListOf<String>()
  var settled: Long = 0

  @OnEvent({"id":"audit-log"})
  @Order(-10)
  fun record(order: PlacedOrder) { log.add(order.status) }

  @OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})
  @Order(10)
  fun settle(order: PlacedOrder): SettledOrder {
    return SettledOrder(order.total)
  }

  @OnEvent
  fun saveSettled(order: SettledOrder) { settled = order.total }
}
