from models import PlacedOrder, SettledOrder


@service
class AuditService:
    log: list[str] = []
    settled = 0
    audited = 0

    @on_event({"id":"audit-log"})
    @order(-10)
    def record(self, order: PlacedOrder) -> None:
        self.log = [*self.log, order.status]

    @on_event({"condition":"value.status == 'paid' && value.total >= 10"})
    @order(10)
    def settle(self, order: PlacedOrder) -> SettledOrder:
        return SettledOrder(order.total)

    @on_event
    def save_settled(self, settled: SettledOrder) -> None:
        self.settled = settled.total

    @on_event
    @InBackground
    async def audit(self, settled: SettledOrder) -> None:
        self.audited = settled.total
