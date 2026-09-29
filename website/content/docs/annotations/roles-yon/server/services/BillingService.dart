@Service
class BillingService {
  static const int _subtotal = 40;

  @Needs
  final BillingHelper helper;

  int total() {
    return helper.settle(_subtotal);
  }
}
