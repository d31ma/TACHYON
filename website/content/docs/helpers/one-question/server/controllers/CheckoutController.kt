import models.Receipt

@RequestRoute("/checkout")
@Public
class CheckoutController {
  @Needs
  val service: CheckoutService

  @GetRoute
  fun getCheckout(): Receipt {
    val total = service.total()
    return Receipt(total)
  }
}
