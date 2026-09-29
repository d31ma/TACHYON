[Helper]
[Lifetime("transient")]
class LedgerHelper {
  [Needs]
  LedgerAmountsHelper _amounts;

  public int settle(int subtotal) {
    int surcharge = _amounts.surcharge();
    return subtotal + surcharge;
  }
}
