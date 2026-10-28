@Service
class CheckoutService {
  @Needs
  final TaxDelegate helper;

  int total() {
    final subtotal = _subtotal();
    return helper.addTax(subtotal);
  }

  int _subtotal() {
    return 40;
  }
}
