@Service
export default class BillingService {
  static _SUBTOTAL = 40;

  /** @type {BillingHelper} */
  @Needs
  billingHelper;

  /** @returns {number} */
  get total() {
    return this.billingHelper.settle(BillingService._SUBTOTAL);
  }
}
