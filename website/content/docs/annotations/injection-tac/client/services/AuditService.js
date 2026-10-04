@Service
export default class AuditService {
  /** @type {CartHelper} */
  @Needs
  @Named("CartHelper")
  ledger;

  /** @returns {number} */
  lines() {
    return this.ledger.count();
  }
}
