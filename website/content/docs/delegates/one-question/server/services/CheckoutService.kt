@Service
class CheckoutService {
  @Needs
  val helper: TaxDelegate

  fun total(): Int {
    val subtotal = subtotal()
    return helper.addTax(subtotal)
  }

  private fun subtotal(): Int = 40
}
