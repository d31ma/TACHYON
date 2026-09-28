<?php
#[Service]
class LedgerService {
  #[Needs]
  private LedgerStore $repository;
  public function subtotal(): int { return $this->repository->subtotal(); }
}
