@Service
export default class AuditService {
  /** @type {CartDelegate} */
  @Needs
  @Named("CartDelegate")
  ledger;

  /** @returns {number} */
  lines() {
    return this.ledger.count();
  }
}
