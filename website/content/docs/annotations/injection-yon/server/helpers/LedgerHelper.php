<?php
#[Helper]
#[Lifetime('transient')]
class LedgerHelper {
  #[Needs]
  private LedgerAmountsHelper $amounts;

  public function settle(int $subtotal): int {
    $surcharge = $this->amounts->surcharge();
    return $subtotal + $surcharge;
  }
}
