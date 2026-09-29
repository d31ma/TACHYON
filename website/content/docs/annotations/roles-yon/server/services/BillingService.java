@Service
class BillingService {
  private static final int SUBTOTAL = 40;

  @Needs
  BillingHelper helper;

  int total() {
    return helper.settle(SUBTOTAL);
  }
}
