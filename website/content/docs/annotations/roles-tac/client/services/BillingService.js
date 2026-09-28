@Service
export default class BillingService {
  /** @type {BillingStore} */
  @Needs
  billingStore;
  /** @type {BillingHelper} */
  @Needs
  billingHelper;

  /** @returns {number} */
  get total() {
    const subtotal = this.billingStore.subtotal();
    return this.billingHelper.settle(subtotal);
  }
}
