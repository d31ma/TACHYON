import models.Product

@Service
class InventoryService {
  var products: List<Product> = listOf(Product("Anvil"), Product("Rope"))
}
