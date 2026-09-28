@Store
@Lifetime('singleton')
class LedgerStore {
  /** @returns {number} */
  subtotal() { return 40; }
  /** @returns {number} */
  surcharge() { return 2; }
}
