import java.util.List
import models.Product

@Service
@Lifetime("transient")
class ProductsService {
  List<Product> list() { return List.of(find(1)) }
  Product find(long id) { return new Product(id, "Anvil") }
}
