@Service
export default class BillingService {
  private static readonly SUBTOTAL = 40;

  @Needs
  helper!: BillingDelegate;

  total(): number {
    return this.helper.settle(BillingService.SUBTOTAL);
  }
}
