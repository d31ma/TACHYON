@Service
class BillingService {
  static const int _subtotal = 40;

  @Needs
  final BillingHelper billingHelper;

  int get total {
    return billingHelper.settle(_subtotal);
  }
}
