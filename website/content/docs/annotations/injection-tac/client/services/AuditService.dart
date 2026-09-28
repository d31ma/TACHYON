@Service
class AuditService {
  @Needs
  @Named("CartStore")
  final CartStore ledger;

  int get lines => ledger.count();
}
