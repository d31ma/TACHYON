using Models;

[RequestRoute("/checkout")]
[Public]
class CheckoutController {
  [Needs]
  CheckoutService _service;

  [GetRoute]
  public Receipt GetCheckout() {
    int total = _service.total();
    return new Receipt(total);
  }
}
