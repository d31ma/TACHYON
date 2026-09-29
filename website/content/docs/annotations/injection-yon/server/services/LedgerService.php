<?php
#[Service]
class LedgerService {
  #[Needs]
  private LedgerAmountsHelper $amounts;
  public function subtotal(): int { return $this->amounts->subtotal(); }
}
