@Service
export default class CheckoutService {
  @Needs
  helper!: TaxDelegate;

  total(): number {
    const subtotal = this.subtotal();
    return this.helper.addTax(subtotal);
  }

  private subtotal(): number {
    return 40;
  }
}
