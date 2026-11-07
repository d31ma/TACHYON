@Delegate
class TaxDelegate {
  @Needs
  final TaxService taxService;

  int rate(String region) => taxService.rate(region) * 2;

  int totalWith(int count, int extra) => taxService.total(count) + extra;
}
