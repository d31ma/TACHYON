@Service
class LedgerService {
  @Needs
  repository: LedgerStore;
  subtotal(): number { return this.repository.subtotal(); }
}
