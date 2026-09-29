@Service
export default class AuditService {
  /** @type {CartPricesHelper} */
  @Needs
  @Named("CartPricesHelper")
  ledger;
  /** @returns {number} */
  get lines() { return this.ledger.count(); }
}
