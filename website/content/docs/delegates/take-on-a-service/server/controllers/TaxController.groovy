import models.TaxAnswer

@RequestRoute("/tax")
@Public
class TaxController {
  @Needs
  TaxDelegate taxDelegate

  @GetRoute
  TaxAnswer getTax() {
    return new TaxAnswer(
        taxDelegate.rate("north"),
        taxDelegate.total(3),
        taxDelegate.total(3, 4))
  }
}
