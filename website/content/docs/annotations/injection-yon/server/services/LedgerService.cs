[Service]
class LedgerService {
  [Needs]
  LedgerAmountsDelegate _amounts;
  public int subtotal() { return _amounts.subtotal(); }
}
