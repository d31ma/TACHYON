[Helper]
[Lifetime("transient")]
class LedgerHelper {
  [Needs]
  LedgerStore _repository;

  public int settle(int subtotal) {
    int surcharge = _repository.surcharge();
    return subtotal + surcharge;
  }
}
