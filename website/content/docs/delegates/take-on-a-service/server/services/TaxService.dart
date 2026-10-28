@Service
class TaxService {
  int rate(String region) => 2;

  int total(int count) => count * 10;
}
