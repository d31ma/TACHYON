import type { Receipt } from '/models/Receipt.ts'

@RequestRoute("/checkout")
@Public
class CheckoutController {
  @Needs
  service: CheckoutService;

  @GetRoute
  getCheckout(): Receipt {
    const total = this.service.total();
    return { total };
  }
}
