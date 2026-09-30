using System.Collections.Generic;
using Models;

[Service]
public class OrdersService {
    public List<string> Log = new List<string>();
    public int Settled = 0;
    public long Audited = 0;
    private readonly ApplicationEventPublisher applicationEventPublisher;

    public void Place(long total) {
        applicationEventPublisher.PublishEvent(new PlacedOrder("paid", total));
    }

    [OnEvent({"id":"orders-log"})]
    [Order(-10)]
    public void Record(PlacedOrder order) { Log.Add(order.Status); }

    [OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})]
    public SettledOrder SettleLargeOrder(PlacedOrder order) {
        Settled += 1;
        return new SettledOrder(order.Total);
    }

    [OnEvent]
    [InBackground]
    public void Audit(SettledOrder order) { Audited = order.Total; }
}
