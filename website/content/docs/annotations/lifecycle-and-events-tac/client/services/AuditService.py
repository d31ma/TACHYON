from models import PlacedOrder, SettledOrder


@service
class AuditService:
    settled: int = 0
    _records: int = 0
    _audited: int = 0

    @on_event({"id":"audit-log","replay":True})
    @order(-10)
    def record(self, order: PlacedOrder) -> None:
        self._records += 1

    @on_event({"condition":"value.status == 'paid' && value.total >= 10"})
    @order(10)
    def settle(self, order: PlacedOrder) -> SettledOrder:
        return SettledOrder(order.total)

    @on_event
    def save_settled(self, order: SettledOrder) -> None:
        self.settled = order.total

    @on_event
    @InBackground
    def audit(self, order: SettledOrder) -> None:
        self._audited = order.total
