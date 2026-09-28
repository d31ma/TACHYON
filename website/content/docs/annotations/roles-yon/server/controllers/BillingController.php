<?php
use Models\Bill;

#[RequestRoute("/billing")]
#[Public]
class BillingController {
  #[Needs]
  private BillingService $service;

  #[GetRoute]
  public function getBilling(): Bill {
    $total = $this->service->total();
    return new Bill($total);
  }
}
