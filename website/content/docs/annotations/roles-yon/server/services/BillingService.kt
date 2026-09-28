@Service
class BillingService {
  @Needs
  val repository: BillingStore
  @Needs
  val helper: BillingHelper

  fun total(): Int {
    val subtotal = repository.subtotal()
    return helper.settle(subtotal)
  }
}
