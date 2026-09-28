@Service
export default class BillingService {
  @Needs
  billingStore!: BillingStore;
  @Needs
  billingHelper!: BillingHelper;

  get total(): number {
    const subtotal = this.billingStore.subtotal();
    return this.billingHelper.settle(subtotal);
  }
}
