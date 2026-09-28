<?php
#[Helper]
class BillingHelper {
  #[Needs]
  private BillingClient $client;

  public function settle(int $subtotal): int {
    $surcharge = $this->client->surcharge();
    return $subtotal + $surcharge;
  }
}
