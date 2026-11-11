<?php
#[Schedule({"fixedDelaySeconds":300})]
class Handler {
  #[Needs]
  private LedgerService $ledger;

  public function POST(): int { return $this->ledger->total(); }
}
