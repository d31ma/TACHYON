/** @import { TaxAnswer } from '/models/TaxAnswer.ts' */
@RequestRoute("/tax")
@Public
class TaxController {
  static _COUNT = 3;

  /** @type {TaxDelegate} */
  @Needs
  taxDelegate;

  /** @returns {TaxAnswer} */
  @GetRoute
  getTax() {
    return {
      rate: this.taxDelegate.rate("north"),
      total: this.taxDelegate.total(TaxController._COUNT),
      extra: this.taxDelegate.total(TaxController._COUNT, 4),
    };
  }
}
