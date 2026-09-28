<?php
#[Helper]
class TaxHelper {
  #[Needs]
  private TaxRateStore $store;

  public function addTax(int $subtotal): int {
    $tax = $this->store->rate();
    return $subtotal + $tax;
  }
}
