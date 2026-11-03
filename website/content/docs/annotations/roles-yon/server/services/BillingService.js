@Service
export default class BillingService {
  static _SUBTOTAL = 40;

  /** @type {BillingDelegate} */
  @Needs
  billingDelegate;

  /** @returns {number} */
  total() {
    return this.billingDelegate.settle(BillingService._SUBTOTAL);
  }
}
