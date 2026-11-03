[Delegate]
class TaxDelegate {
  [Needs]
  TaxService _taxService;

  public int Rate(string region) {
    return _taxService.Rate(region) * 2;
  }

  public int Total(int count, int extra) {
    return _taxService.Total(count) + extra;
  }
}
