<?php
use Models\Receipt;

#[RequestRoute("/checkout")]
#[Public]
class CheckoutController {
  #[Needs]
  private CheckoutService $service;

  #[GetRoute]
  public function getCheckout(): Receipt {
    $total = $this->service->total();
    return new Receipt($total);
  }
}
