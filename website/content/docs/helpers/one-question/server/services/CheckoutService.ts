@Service
class CheckoutService {
  @Needs
  helper: TaxHelper;

  total(): number {
    const subtotal = this.subtotal();
    return this.helper.addTax(subtotal);
  }

  private subtotal(): number {
    return 40;
  }
}
