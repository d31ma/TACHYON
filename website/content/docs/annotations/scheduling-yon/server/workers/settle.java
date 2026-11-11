@Schedule({"fixedDelaySeconds":300})
class Handler {
  @Needs
  LedgerService ledger;

  int POST() { return ledger.total(); }
}
