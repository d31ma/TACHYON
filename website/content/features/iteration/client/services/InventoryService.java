import java.util.List;
import models.Product;

@Service
class InventoryService {
    List<Product> products = List.of(new Product("Anvil"), new Product("Rope"));
}
