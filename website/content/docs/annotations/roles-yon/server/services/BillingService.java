@Service
class BillingService {
  private static final int SUBTOTAL = 40;

  @Needs
  BillingDelegate helper;

  int total() {
    return helper.settle(SUBTOTAL);
  }
}
