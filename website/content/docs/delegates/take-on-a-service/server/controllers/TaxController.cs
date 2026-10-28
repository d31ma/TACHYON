using Models;

[RequestRoute("/tax")]
[Public]
class TaxController {
  private const int Count = 3;

  [Needs]
  TaxDelegate _taxDelegate;

  [GetRoute]
  public TaxAnswer GetTax() {
    return new TaxAnswer(
      _taxDelegate.Rate("north"),
      _taxDelegate.Total(Count),
      _taxDelegate.Total(Count, 4));
  }
}
