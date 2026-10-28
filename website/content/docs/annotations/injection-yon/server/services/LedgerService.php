<?php
#[Service]
class LedgerService {
  #[Needs]
  private LedgerAmountsDelegate $amounts;
  public function subtotal(): int { return $this->amounts->subtotal(); }
}
