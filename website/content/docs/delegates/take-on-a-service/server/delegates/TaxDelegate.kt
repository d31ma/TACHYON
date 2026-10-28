@Delegate
class TaxDelegate {
  @Needs
  val taxService: TaxService

  fun rate(region: String): Int = taxService.rate(region) * 2

  fun total(count: Int, extra: Int): Int = taxService.total(count) + extra
}
