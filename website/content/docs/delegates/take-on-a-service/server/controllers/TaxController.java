import models.TaxAnswer;

@RequestRoute("/tax")
@Public
class TaxController {
  private static final int COUNT = 3;

  @Needs
  TaxDelegate taxDelegate;

  @GetRoute
  TaxAnswer getTax() {
    return new TaxAnswer(
        taxDelegate.rate("north"),
        taxDelegate.total(COUNT),
        taxDelegate.total(COUNT, 4));
  }
}
