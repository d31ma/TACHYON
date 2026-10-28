import models.TaxAnswer

@RequestRoute("/tax")
@Public
class TaxController {
  @Needs
  val taxDelegate: TaxDelegate

  @GetRoute
  fun getTax(): TaxAnswer = TaxAnswer(
    rate = taxDelegate.rate("north"),
    total = taxDelegate.total(3),
    extra = taxDelegate.total(3, 4),
  )
}
