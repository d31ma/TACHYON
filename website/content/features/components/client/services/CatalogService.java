import java.util.List;
import models.Product;

@Service
class CatalogService {
    final List<Product> products = List.of(
        new Product("anvil", "Anvil", "£42.00"),
        new Product("rope", "Rope", "£8.00"));
}
