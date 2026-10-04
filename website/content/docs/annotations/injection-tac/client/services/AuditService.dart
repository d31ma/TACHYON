@Service
class AuditService {
  @Needs
  @Named("CartHelper")
  final CartHelper ledger;

  int lines() {
    return ledger.count();
  }
}
