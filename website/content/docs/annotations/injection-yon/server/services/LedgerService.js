@Service
export default class LedgerService {
  /** @type {LedgerAmountsDelegate} */
  @Needs
  ledgerAmountsDelegate;
  /** @returns {number} */
  subtotal() { return this.ledgerAmountsDelegate.subtotal(); }
}
