@Helper
export default class BillingHelper {
  static readonly SURCHARGE: number = 2;

  settle(subtotal: number): number {
    return subtotal + BillingHelper.SURCHARGE;
  }
}
