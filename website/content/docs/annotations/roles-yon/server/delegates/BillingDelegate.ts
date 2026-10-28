@Delegate
export default class BillingDelegate {
  private static readonly SURCHARGE: number = 2;

  settle(subtotal: number): number {
    return subtotal + BillingDelegate.SURCHARGE;
  }
}
