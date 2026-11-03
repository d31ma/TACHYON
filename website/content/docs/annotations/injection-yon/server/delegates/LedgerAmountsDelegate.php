<?php
#[Delegate]
#[Lifetime('singleton')]
class LedgerAmountsDelegate {
  public function subtotal(): int { return 40; }
  public function surcharge(): int { return 2; }
}
