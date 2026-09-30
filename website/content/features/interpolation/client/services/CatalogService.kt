import models.Product

@Service
class CatalogService {
  var title: String = "Catalog"
  var count: Int = 6
  var product: Product = Product("Anvil", "£42.00")
}
