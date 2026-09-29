<?php
#[Helper]
class TaxHelper {
  #[Needs]
  private TaxRateHelper $rates;

  public function addTax(int $subtotal): int {
    $tax = $this->rates->rate();
    return $subtotal + $tax;
  }
}
