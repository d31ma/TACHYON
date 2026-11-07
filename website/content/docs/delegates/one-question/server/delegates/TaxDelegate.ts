@Delegate
export default class TaxDelegate {
  @Needs
  rates!: TaxRateDelegate;

  addTax(subtotal: number): number {
    const tax = this.rates.rate();
    return subtotal + tax;
  }
}
