@Helper
export default class TaxHelper {
  @Needs
  store!: TaxRateStore;

  addTax(subtotal: number): number {
    const tax = this.store.rate();
    return subtotal + tax;
  }
}
