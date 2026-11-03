<?php
#[Delegate]
#[Lifetime('transient')]
class LedgerDelegate {
  #[Needs]
  private LedgerAmountsDelegate $amounts;

  public function settle(int $subtotal): int {
    $surcharge = $this->amounts->surcharge();
    return $subtotal + $surcharge;
  }
}
