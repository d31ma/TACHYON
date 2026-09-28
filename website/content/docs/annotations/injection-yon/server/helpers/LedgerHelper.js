@Helper
@Lifetime('transient')
class LedgerHelper {
  /** @type {LedgerStore} */
  @Needs
  ledgerStore;

  /**
   * @param {number} subtotal
   * @returns {number}
   */
  settle(subtotal) {
    const surcharge = this.ledgerStore.surcharge();
    return subtotal + surcharge;
  }
}
