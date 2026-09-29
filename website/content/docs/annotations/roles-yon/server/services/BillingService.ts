@Service
export default class BillingService {
  private static readonly SUBTOTAL = 40;

  @Needs
  helper!: BillingHelper;

  total(): number {
    return this.helper.settle(BillingService.SUBTOTAL);
  }
}
