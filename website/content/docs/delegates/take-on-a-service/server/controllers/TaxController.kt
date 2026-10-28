import models.TaxAnswer

@RequestRoute("/tax")
@Public
class TaxController {
  @Needs
  val taxDelegate: TaxDelegate

  @GetRoute
  fun getTax(): TaxAnswer = TaxAnswer(
    rate = taxDelegate.rate("north"),
    total = taxDelegate.total(COUNT),
    extra = taxDelegate.total(COUNT, 4),
  )

  companion object {
    private const val COUNT = 3
  }
}
