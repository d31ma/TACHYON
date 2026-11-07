@Delegate
class TaxDelegate {
  @Needs
  TaxService taxService

  int rate(String region) {
    return taxService.rate(region) * 2
  }

  int total(int count, int extra) {
    return taxService.total(count) + extra
  }
}
