@Helper
@Lifetime("transient")
class LedgerHelper {
  @Needs
  LedgerAmountsHelper amounts

  int settle(int subtotal) {
    int surcharge = amounts.surcharge()
    return subtotal + surcharge
  }
}
