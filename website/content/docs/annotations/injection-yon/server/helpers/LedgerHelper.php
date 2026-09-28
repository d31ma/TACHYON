<?php
#[Helper]
#[Lifetime('transient')]
class LedgerHelper {
  #[Needs]
  private LedgerStore $repository;

  public function settle(int $subtotal): int {
    $surcharge = $this->repository->surcharge();
    return $subtotal + $surcharge;
  }
}
