[Service]
class LedgerService {
  [Needs]
  LedgerStore _repository;
  public int subtotal() { return _repository.subtotal(); }
}
