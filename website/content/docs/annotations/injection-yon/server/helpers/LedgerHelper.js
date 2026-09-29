@Helper
@Lifetime('transient')
export default class LedgerHelper {
  /** @type {LedgerAmountsHelper} */
  @Needs
  ledgerAmountsHelper;

  /**
   * @param {number} subtotal
   * @returns {number}
   */
  settle(subtotal) {
    const surcharge = this.ledgerAmountsHelper.surcharge();
    return subtotal + surcharge;
  }
}
