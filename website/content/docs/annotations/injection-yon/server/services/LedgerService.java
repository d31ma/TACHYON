@Service
class LedgerService {
  @Needs
  LedgerStore repository;
  int subtotal() { return repository.subtotal(); }
}
