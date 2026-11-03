import models.Product

@Service
@Lifetime("transient")
class ProductsService {
  fun list(): List<Product> = listOf(find(1))
  fun find(id: Long): Product = Product(id, "Anvil")
}
