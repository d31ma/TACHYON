@Service
class BillingService {
  static const int _subtotal = 40;

  @Needs
  final BillingDelegate helper;

  int total() {
    return helper.settle(_subtotal);
  }
}
