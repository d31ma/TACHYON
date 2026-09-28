import models.PlacedOrder
import models.SettledOrder

class AuditService {
    private var records: Int = 0
    var settled: Long = 0
    private var audited: Long = 0

    @OnEvent({"id":"audit-log","replay":true})
    @Order(-10)
    fun record(order: PlacedOrder) { records += 1 }

    @OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})
    @Order(10)
    fun settle(order: PlacedOrder): SettledOrder {
        return SettledOrder(order.total)
    }

    @OnEvent
    fun saveSettled(order: SettledOrder) { settled = order.total }

    @OnEvent
    @InBackground
    fun audit(order: SettledOrder) { audited = order.total }
}
