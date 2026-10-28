@Delegate
@Lifetime("transient")
class LedgerDelegate {
  @Needs
  LedgerAmountsDelegate amounts;

  int settle(int subtotal) {
    int surcharge = amounts.surcharge();
    return subtotal + surcharge;
  }
}
