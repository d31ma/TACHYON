@Service
export default class AuditService {
  @Needs
  @Named("CartHelper")
  ledger!: CartHelper;

  lines(): number {
    return this.ledger.count();
  }
}
