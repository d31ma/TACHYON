import models.Product

@Service
class StockService {
  var products: List<Product> = listOf()
  var restocking: Boolean = true
}
