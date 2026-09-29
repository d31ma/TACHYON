@Service
class LedgerService {
  @Needs
  final LedgerAmountsHelper amounts;

  int subtotal() {
    return amounts.subtotal();
  }
}
