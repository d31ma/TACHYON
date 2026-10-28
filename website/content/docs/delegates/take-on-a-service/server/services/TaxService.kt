@Service
class TaxService {
  fun rate(region: String): Int = 2

  fun total(count: Int): Int = count * 10
}
