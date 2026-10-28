<?php
#[Service]
class BillingService {
  #[Needs]
  private BillingDelegate $helper;

  public function total(): int {
    return $this->helper->settle($this->subtotal());
  }

  private function subtotal(): int {
    return 40;
  }
}
