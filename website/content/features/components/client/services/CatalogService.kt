import models.Product

@Service
class CatalogService {
  val products: List<Product> = listOf(
    Product("anvil", "Anvil", "£42.00"),
    Product("rope", "Rope", "£8.00"),
  )
}
