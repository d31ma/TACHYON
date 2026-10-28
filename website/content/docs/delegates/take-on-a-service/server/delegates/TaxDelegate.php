<?php
#[Delegate]
class TaxDelegate {
  #[Needs]
  private TaxService $taxService;

  public function rate(string $region): int {
    return $this->taxService->rate($region) * 2;
  }

  public function total(int $count, int $extra): int {
    return $this->taxService->total($count) + $extra;
  }
}
