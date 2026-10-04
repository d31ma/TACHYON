from models import PlacedOrder, SettledOrder


@service
class OrdersService:
    application_event_publisher: ApplicationEventPublisher
    log: list[str] = []
    settled: int = 0
    audited: int = 0

    def place(self, total: int) -> None:
        self.application_event_publisher.publish_event(PlacedOrder("paid", total), False)

    @on_event({"id":"orders-log"})
    @order(-10)
    def record(self, order: PlacedOrder) -> None:
        self.log = [*self.log, order.status]

    @on_event({"condition":"value.status == 'paid' && value.total >= 10"})
    def settle_large_order(self, order: PlacedOrder) -> SettledOrder:
        self.settled += 1
        return SettledOrder(order.total)

    @on_event
    @InBackground
    def audit(self, settled: SettledOrder) -> None:
        self.audited = settled.total
