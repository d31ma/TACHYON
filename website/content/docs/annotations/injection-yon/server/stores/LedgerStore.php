<?php
#[Store]
#[Lifetime('singleton')]
class LedgerStore {
  public function subtotal(): int { return 40; }
  public function surcharge(): int { return 2; }
}
