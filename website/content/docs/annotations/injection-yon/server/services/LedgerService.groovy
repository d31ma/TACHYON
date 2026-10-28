@Service
class LedgerService {
  @Needs
  LedgerAmountsDelegate amounts
  int subtotal() { return amounts.subtotal() }
}
