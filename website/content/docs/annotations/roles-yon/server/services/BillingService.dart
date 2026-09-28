@Service
class BillingService {
  @Needs
  final BillingStore repository;
  @Needs
  final BillingHelper helper;

  int total() {
    final subtotal = repository.subtotal();
    return helper.settle(subtotal);
  }
}
