/** @import { Bill } from '/models/Bill.ts' */
@RequestRoute("/billing")
@Public
class BillingController {
  /** @type {BillingService} */
  @Needs
  billingService;

  /** @returns {Bill} */
  @GetRoute
  getBilling() {
    const total = this.billingService.total();
    return { total };
  }
}
