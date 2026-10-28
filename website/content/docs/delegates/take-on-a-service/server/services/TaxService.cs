[Service]
class TaxService {
  public int Rate(string region) {
    return 2;
  }

  public int Total(int count) {
    return count * 10;
  }
}
