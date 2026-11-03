@Service
export default class LedgerService {
  @Needs
  amounts!: LedgerAmountsDelegate;
  subtotal(): number { return this.amounts.subtotal(); }
}
