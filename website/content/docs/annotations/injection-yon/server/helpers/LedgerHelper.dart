@Helper
@Lifetime("transient")
class LedgerHelper {
  @Needs
  final LedgerAmountsHelper amounts;

  int settle(int subtotal) {
    final surcharge = amounts.surcharge();
    return subtotal + surcharge;
  }
}
