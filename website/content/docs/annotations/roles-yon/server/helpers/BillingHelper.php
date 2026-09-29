<?php
#[Helper]
class BillingHelper {
  private const SURCHARGE = 2;

  public function settle(int $subtotal): int {
    return $subtotal + self::SURCHARGE;
  }
}
