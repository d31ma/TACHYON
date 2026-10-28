<?php
#[Delegate]
class TaxDelegate {
  #[Needs]
  private TaxRateDelegate $rates;

  public function addTax(int $subtotal): int {
    $tax = $this->rates->rate();
    return $subtotal + $tax;
  }
}
