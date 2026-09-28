@Service
class CheckoutService {
  @Needs
  TaxHelper helper

  int total() {
    int subtotal = subtotal()
    return helper.addTax(subtotal)
  }

  private int subtotal() {
    return 40
  }
}
