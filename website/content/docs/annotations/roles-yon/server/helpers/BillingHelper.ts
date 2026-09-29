@Helper
export default class BillingHelper {
  @Needs
  client!: BillingClient;

  settle(subtotal: number): number {
    const surcharge = this.client.surcharge();
    return subtotal + surcharge;
  }
}
