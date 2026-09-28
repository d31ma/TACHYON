/** @import { Receipt } from '/models/Receipt.ts' */
@RequestRoute("/checkout")
@Public
class CheckoutController {
  /** @type {CheckoutService} */
  @Needs
  checkoutService;

  /** @returns {Receipt} */
  @GetRoute
  getCheckout() {
    const total = this.checkoutService.total();
    return { total };
  }
}
