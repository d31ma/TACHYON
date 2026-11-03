[Delegate]
[Lifetime("transient")]
class LedgerDelegate {
  [Needs]
  LedgerAmountsDelegate _amounts;

  public int settle(int subtotal) {
    int surcharge = _amounts.surcharge();
    return subtotal + surcharge;
  }
}
