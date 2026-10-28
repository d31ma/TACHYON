@Service
export default class AuditService {
  @Needs
  @Named("CartDelegate")
  ledger!: CartDelegate;

  lines(): number {
    return this.ledger.count();
  }
}
