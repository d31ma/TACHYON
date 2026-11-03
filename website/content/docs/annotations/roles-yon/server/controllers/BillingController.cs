using Models;

[RequestRoute("/billing")]
[Public]
class BillingController {
  [Needs]
  BillingService _service;

  [GetRoute]
  public Bill GetBilling() {
    int total = _service.total();
    return new Bill(total);
  }
}
