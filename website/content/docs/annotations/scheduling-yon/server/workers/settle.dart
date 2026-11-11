@Schedule({"fixedDelaySeconds":300})
class Handler {
  @Needs
  final LedgerService ledger;

  int POST() {
    return ledger.total();
  }
}
