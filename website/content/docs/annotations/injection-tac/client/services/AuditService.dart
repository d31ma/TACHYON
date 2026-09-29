@Service
class AuditService {
  @Needs
  @Named("CartPricesHelper")
  final CartPricesHelper ledger;

  int get lines => ledger.count();
}
