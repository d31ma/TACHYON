@Helper
export default class TaxHelper {
  @Needs
  rates!: TaxRateHelper;

  addTax(subtotal: number): number {
    const tax = this.rates.rate();
    return subtotal + tax;
  }
}
