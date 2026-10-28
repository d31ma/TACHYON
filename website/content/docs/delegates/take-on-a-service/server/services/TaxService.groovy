@Service
class TaxService {
  int rate(String region) {
    return 2
  }

  int total(int count) {
    return count * 10
  }
}
