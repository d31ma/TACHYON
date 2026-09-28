@Service
class BillingService {
  @Needs
  BillingStore repository
  @Needs
  BillingHelper helper

  int total() {
    int subtotal = repository.subtotal()
    return helper.settle(subtotal)
  }
}
