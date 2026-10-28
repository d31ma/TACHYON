<?php
use Models\TaxAnswer;

#[RequestRoute("/tax")]
#[Public]
class TaxController {
  #[Needs]
  private TaxDelegate $taxDelegate;

  #[GetRoute]
  public function getTax(): TaxAnswer {
    return new TaxAnswer(
      $this->taxDelegate->rate("north"),
      $this->taxDelegate->total(3),
      $this->taxDelegate->total(3, 4),
    );
  }
}
