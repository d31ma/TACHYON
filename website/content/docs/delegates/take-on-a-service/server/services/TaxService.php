<?php
#[Service]
class TaxService {
  public function rate(string $region): int {
    return 2;
  }

  public function total(int $count): int {
    return $count * 10;
  }
}
