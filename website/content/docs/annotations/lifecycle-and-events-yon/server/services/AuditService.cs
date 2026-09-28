[Service]
class AuditService {
  public readonly System.Collections.Generic.List<string> Log = new();
  public long Settled = 0;

  [OnEvent({"id":"audit-log"})]
  [Order(-10)]
  public void Record(Models.PlacedOrder order) { Log.Add(order.Status); }

  [OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})]
  [Order(10)]
  public Models.SettledOrder Settle(Models.PlacedOrder order) {
    return new Models.SettledOrder(order.Total);
  }

  [OnEvent]
  public void SaveSettled(Models.SettledOrder order) { Settled = order.Total; }
}
