@Service
final class OrdersService {
    let applicationEventPublisher: ApplicationEventPublisher
    var log: [String] = []
    var settled: Int = 0
    var audited: Int = 0

    func place(_ total: Int) {
        applicationEventPublisher.publishEvent(PlacedOrder(status: "paid", total: total))
    }

    @OnEvent({"id":"orders-log"})
    @Order(-10)
    func record(_ order: PlacedOrder) { log.append(order.status) }

    @OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})
    func settleLargeOrder(_ order: PlacedOrder) -> SettledOrder {
        settled += 1
        return SettledOrder(total: order.total)
    }

    @OnEvent
    @InBackground
    func audit(_ order: SettledOrder) { audited = order.total }
}
