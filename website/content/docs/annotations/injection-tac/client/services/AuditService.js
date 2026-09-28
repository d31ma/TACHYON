@Service
export default class AuditService {
  /** @type {CartStore} */
  @Needs
  @Named("CartStore")
  ledger;
  /** @returns {number} */
  get lines() { return this.ledger.count(); }
}
