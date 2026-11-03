@Delegate
@Lifetime("transient")
class LedgerDelegate {
  @Needs
  final LedgerAmountsDelegate amounts;

  int settle(int subtotal) {
    final surcharge = amounts.surcharge();
    return subtotal + surcharge;
  }
}
