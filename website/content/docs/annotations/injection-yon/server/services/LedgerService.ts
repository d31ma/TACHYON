@Service
export default class LedgerService {
  @Needs
  amounts!: LedgerAmountsHelper;
  subtotal(): number { return this.amounts.subtotal(); }
}
