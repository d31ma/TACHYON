@Service
export default class BillingService {
  static _SUBTOTAL = 40;

  /** @type {BillingDelegate} */
  @Needs
  billingDelegate;

  /** @returns {number} */
  get total() {
    return this.billingDelegate.settle(BillingService._SUBTOTAL);
  }
}
