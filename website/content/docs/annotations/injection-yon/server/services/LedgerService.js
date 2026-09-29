@Service
export default class LedgerService {
  /** @type {LedgerStore} */
  @Needs
  ledgerStore;
  /** @returns {number} */
  subtotal() { return this.ledgerStore.subtotal(); }
}
