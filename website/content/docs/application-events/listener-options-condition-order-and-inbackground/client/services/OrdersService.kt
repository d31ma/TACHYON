import models.PlacedOrder
import models.SettledOrder

@Service
class OrdersService {
    val applicationEventPublisher: ApplicationEventPublisher
    var log: List<String> = listOf()
    var settled: Int = 0
    var audited: Long = 0

    fun place(total: Long) {
        applicationEventPublisher.publishEvent(PlacedOrder("paid", total))
    }

    @OnEvent({"id":"orders-log"})
    @Order(-10)
    fun record(order: PlacedOrder) { log = log + order.status }

    @OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})
    fun settleLargeOrder(order: PlacedOrder): SettledOrder {
        settled += 1
        return SettledOrder(order.total)
    }

    @OnEvent
    @InBackground
    fun audit(order: SettledOrder) { audited = order.total }
}
