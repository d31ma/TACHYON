<?php
#[Service]
class BillingService {
  #[Needs]
  private BillingStore $repository;
  #[Needs]
  private BillingHelper $helper;

  public function total(): int {
    $subtotal = $this->repository->subtotal();
    return $this->helper->settle($subtotal);
  }
}
