@Service
class BillingService {
  @Needs
  val helper: BillingDelegate

  fun total(): Int = helper.settle(SUBTOTAL)

  companion object {
    private const val SUBTOTAL = 40
  }
}
