@Service
class CheckoutService {
  @Needs
  TaxDelegate helper;

  int total() {
    int subtotal = subtotal();
    return helper.addTax(subtotal);
  }

  private int subtotal() {
    return 40;
  }
}
