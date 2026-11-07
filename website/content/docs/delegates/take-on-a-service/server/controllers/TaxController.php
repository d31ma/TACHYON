<?php
use Models\TaxAnswer;

#[RequestRoute("/tax")]
#[Public]
class TaxController {
  private const int COUNT = 3;

  #[Needs]
  private TaxDelegate $taxDelegate;

  #[GetRoute]
  public function getTax(): TaxAnswer {
    return new TaxAnswer(
      $this->taxDelegate->rate("north"),
      $this->taxDelegate->total(self::COUNT),
      $this->taxDelegate->total(self::COUNT, 4),
    );
  }
}
