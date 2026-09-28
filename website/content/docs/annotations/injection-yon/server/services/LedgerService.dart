@Service
class LedgerService {
  @Needs
  final LedgerStore repository;

  int subtotal() {
    return repository.subtotal();
  }
}
