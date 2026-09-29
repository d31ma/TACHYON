@Service
export default class BillingService {
  private static readonly SUBTOTAL = 40;

  @Needs
  billingHelper!: BillingHelper;

  get total(): number {
    return this.billingHelper.settle(BillingService.SUBTOTAL);
  }
}
