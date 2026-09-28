using Models;

public class AuditService {
    private int Records = 0;
    public long Settled = 0;
    private long Audited = 0;

    [OnEvent({"id":"audit-log","replay":true})]
    [Order(-10)]
    public void Record(PlacedOrder order) { Records += 1; }

    [OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})]
    [Order(10)]
    public SettledOrder Settle(PlacedOrder order) { return new SettledOrder(order.Total); }

    [OnEvent]
    public void SaveSettled(SettledOrder order) { Settled = order.Total; }

    [OnEvent]
    [InBackground]
    public void Audit(SettledOrder order) { Audited = order.Total; }
}
