final class AuditService {
    private var records: Int = 0
    var settled: Int = 0
    private var audited: Int = 0

    @OnEvent({"id":"audit-log","replay":true})
    @Order(-10)
    func record(_ order: PlacedOrder) { records += 1 }

    @OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})
    @Order(10)
    func settle(_ order: PlacedOrder) -> SettledOrder { SettledOrder(total: order.total) }

    @OnEvent
    func saveSettled(_ order: SettledOrder) { settled = order.total }

    @OnEvent
    @InBackground
    func audit(_ order: SettledOrder) { audited = order.total }
}
