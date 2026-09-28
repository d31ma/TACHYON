import models.Bill;

@RequestRoute("/billing")
@Public
class BillingController {
  @Needs
  BillingService service;

  @GetRoute
  Bill getBilling() {
    int total = service.total();
    return new Bill(total);
  }
}
