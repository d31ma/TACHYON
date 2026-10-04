import models.PlacedOrder;
import models.SettledOrder;

@Service
class AuditService {
    private int records = 0;
    long settled = 0;
    private long audited = 0;

    @OnEvent({"id":"audit-log","replay":true})
    @Order(-10)
    void record(PlacedOrder order) { records += 1; }

    @OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})
    @Order(10)
    SettledOrder settle(PlacedOrder order) {
        return new SettledOrder(order.total());
    }

    @OnEvent
    void saveSettled(SettledOrder order) { settled = order.total(); }

    @OnEvent
    @InBackground
    void audit(SettledOrder order) { audited = order.total(); }
}
