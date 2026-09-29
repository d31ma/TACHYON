@Service
export default class LedgerService {
  /** @type {LedgerAmountsHelper} */
  @Needs
  ledgerAmountsHelper;
  /** @returns {number} */
  subtotal() { return this.ledgerAmountsHelper.subtotal(); }
}
