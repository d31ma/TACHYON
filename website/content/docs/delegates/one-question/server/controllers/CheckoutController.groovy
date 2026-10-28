import models.Receipt

@RequestRoute("/checkout")
@Public
class CheckoutController {
  @Needs
  CheckoutService service

  @GetRoute
  Receipt getCheckout() {
    int total = service.total()
    return new Receipt(total)
  }
}
