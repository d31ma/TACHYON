@Store
@Lifetime('singleton')
export default class LedgerStore {
  subtotal(): number { return 40; }
  surcharge(): number { return 2; }
}
