[Schedule({"fixedDelaySeconds":300})]
class Handler {
  [Needs]
  LedgerService _ledger;

  public int POST() { return _ledger.Total(); }
}
