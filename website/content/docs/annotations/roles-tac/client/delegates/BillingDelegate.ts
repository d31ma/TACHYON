@Delegate
export default class BillingDelegate {
  settle(subtotal: number): number { return subtotal + 2; }
}
