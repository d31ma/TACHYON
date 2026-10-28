/** @import { TaxAnswer } from '/models/TaxAnswer.ts' */
@RequestRoute("/tax")
@Public
class TaxController {
  /** @type {TaxDelegate} */
  @Needs
  taxDelegate;

  /** @returns {TaxAnswer} */
  @GetRoute
  getTax() {
    return {
      rate: this.taxDelegate.rate("north"),
      total: this.taxDelegate.total(3),
      extra: this.taxDelegate.total(3, 4),
    };
  }
}
