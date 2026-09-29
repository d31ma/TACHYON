<?php
#[Helper]
#[Lifetime('singleton')]
class LedgerAmountsHelper {
  public function subtotal(): int { return 40; }
  public function surcharge(): int { return 2; }
}
