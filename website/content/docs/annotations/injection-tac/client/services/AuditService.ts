@Service
export default class AuditService {
  @Needs
  @Named("CartStore")
  ledger!: CartStore;
  get lines(): number { return this.ledger.count(); }
}
