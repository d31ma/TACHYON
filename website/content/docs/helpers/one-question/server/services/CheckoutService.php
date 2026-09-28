<?php
#[Service]
class CheckoutService {
  #[Needs]
  private TaxHelper $helper;

  public function total(): int {
    $subtotal = $this->subtotal();
    return $this->helper->addTax($subtotal);
  }

  private function subtotal(): int {
    return 40;
  }
}
