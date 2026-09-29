[Service]
class LedgerService {
  [Needs]
  LedgerAmountsHelper _amounts;
  public int subtotal() { return _amounts.subtotal(); }
}
