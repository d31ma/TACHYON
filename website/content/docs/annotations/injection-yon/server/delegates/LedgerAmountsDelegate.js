@Delegate
@Lifetime('singleton')
export default class LedgerAmountsDelegate {
  /** @returns {number} */
  subtotal() { return 40; }
  /** @returns {number} */
  surcharge() { return 2; }
}
