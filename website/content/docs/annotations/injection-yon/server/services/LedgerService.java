@Service
class LedgerService {
  @Needs
  LedgerAmountsHelper amounts;
  int subtotal() { return amounts.subtotal(); }
}
