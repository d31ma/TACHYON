@Delegate
@Lifetime('singleton')
export default class LedgerAmountsDelegate {
  subtotal(): number { return 40; }
  surcharge(): number { return 2; }
}
