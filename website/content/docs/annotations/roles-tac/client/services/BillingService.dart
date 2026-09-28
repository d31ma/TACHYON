@Service
class BillingService {
  @Needs
  final BillingStore billingStore;
  @Needs
  final BillingHelper billingHelper;

  int get total {
    final subtotal = billingStore.subtotal();
    return billingHelper.settle(subtotal);
  }
}
