@Service
export default class AuditService {
  @Needs
  @Named("CartPricesHelper")
  ledger!: CartPricesHelper;
  get lines(): number { return this.ledger.count(); }
}
