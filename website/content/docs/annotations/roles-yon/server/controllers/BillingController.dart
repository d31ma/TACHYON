import 'models/Bill.dart';

@RequestRoute("/billing")
@Public
class BillingController {
  @Needs
  final BillingService service;

  @GetRoute
  Bill getBilling() {
    final total = service.total();
    return Bill(total);
  }
}
