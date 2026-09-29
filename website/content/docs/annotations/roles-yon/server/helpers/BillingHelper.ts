@Helper
export default class BillingHelper {
  private static readonly SURCHARGE: number = 2;

  settle(subtotal: number): number {
    return subtotal + BillingHelper.SURCHARGE;
  }
}
