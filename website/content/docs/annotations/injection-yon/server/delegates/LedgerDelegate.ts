@Delegate
@Lifetime('transient')
export default class LedgerDelegate {
  @Needs
  amounts!: LedgerAmountsDelegate;

  settle(subtotal: number): number {
    const surcharge = this.amounts.surcharge();
    return subtotal + surcharge;
  }
}
