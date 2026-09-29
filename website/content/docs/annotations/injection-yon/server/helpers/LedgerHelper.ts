@Helper
@Lifetime('transient')
export default class LedgerHelper {
  @Needs
  amounts!: LedgerAmountsHelper;

  settle(subtotal: number): number {
    const surcharge = this.amounts.surcharge();
    return subtotal + surcharge;
  }
}
