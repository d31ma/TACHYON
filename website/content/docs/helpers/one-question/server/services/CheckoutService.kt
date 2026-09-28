@Service
class CheckoutService {
  @Needs
  val helper: TaxHelper

  fun total(): Int {
    val subtotal = subtotal()
    return helper.addTax(subtotal)
  }

  private fun subtotal(): Int = 40
}
