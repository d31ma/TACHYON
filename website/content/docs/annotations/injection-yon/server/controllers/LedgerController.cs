using Models;

[RequestRoute("/ledger")]
[Public]
class LedgerController {
  [Needs]
  [Named("LedgerService")]
  LedgerService _service;
  [Needs]
  [Named("LedgerDelegate")]
  LedgerDelegate _settlement;

  [GetRoute]
  public Ledger GetLedger() {
    int subtotal = _service.subtotal();
    int total = _settlement.settle(subtotal);
    return new Ledger(total);
  }
}
