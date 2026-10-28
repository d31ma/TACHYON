using Models;

[RequestRoute("/tax")]
[Public]
class TaxController {
  [Needs]
  TaxDelegate _taxDelegate;

  [GetRoute]
  public TaxAnswer GetTax() {
    return new TaxAnswer(
      _taxDelegate.Rate("north"),
      _taxDelegate.Total(3),
      _taxDelegate.Total(3, 4));
  }
}
