@Service
class BillingService {
  @Needs
  repository: BillingStore;
  @Needs
  helper: BillingHelper;

  total(): number {
    const subtotal = this.repository.subtotal();
    return this.helper.settle(subtotal);
  }
}
