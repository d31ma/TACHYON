@Service
class LedgerService {
  @Needs
  final LedgerAmountsDelegate amounts;

  int subtotal() {
    return amounts.subtotal();
  }
}
