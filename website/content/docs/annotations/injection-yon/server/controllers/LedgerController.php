<?php
use Models\Ledger;

#[RequestRoute("/ledger")]
#[Public]
class LedgerController {
  #[Needs]
  #[Named('LedgerService')]
  private LedgerService $service;
  #[Needs]
  #[Named('LedgerHelper')]
  private LedgerHelper $settlement;

  #[GetRoute]
  public function getLedger(): Ledger {
    $subtotal = $this->service->subtotal();
    $total = $this->settlement->settle($subtotal);
    return new Ledger($total);
  }
}
