import 'models/Receipt.dart';

@RequestRoute("/checkout")
@Public
class CheckoutController {
  @Needs
  final CheckoutService service;

  @GetRoute
  Receipt getCheckout() {
    final total = service.total();
    return Receipt(total);
  }
}
