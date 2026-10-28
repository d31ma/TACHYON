@Service
class AuditService {
  @Needs
  @Named("CartDelegate")
  final CartDelegate ledger;

  int lines() {
    return ledger.count();
  }
}
