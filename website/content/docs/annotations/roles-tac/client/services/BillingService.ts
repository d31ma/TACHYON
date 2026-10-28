@Service
export default class BillingService {
  private static readonly SUBTOTAL = 40;

  @Needs
  billingDelegate!: BillingDelegate;

  get total(): number {
    return this.billingDelegate.settle(BillingService.SUBTOTAL);
  }
}
