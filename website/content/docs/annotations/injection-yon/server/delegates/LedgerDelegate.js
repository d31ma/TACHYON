@Delegate
@Lifetime('transient')
export default class LedgerDelegate {
  /** @type {LedgerAmountsDelegate} */
  @Needs
  ledgerAmountsDelegate;

  /**
   * @param {number} subtotal
   * @returns {number}
   */
  settle(subtotal) {
    const surcharge = this.ledgerAmountsDelegate.surcharge();
    return subtotal + surcharge;
  }
}
