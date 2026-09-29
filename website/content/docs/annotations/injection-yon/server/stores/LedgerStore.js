@Store
@Lifetime('singleton')
export default class LedgerStore {
  /** @returns {number} */
  subtotal() { return 40; }
  /** @returns {number} */
  surcharge() { return 2; }
}
