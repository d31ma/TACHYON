@Service
class BillingService {
  static const int _subtotal = 40;

  @Needs
  final BillingDelegate billingDelegate;

  int get total {
    return billingDelegate.settle(_subtotal);
  }
}
