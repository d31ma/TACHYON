import java.util.List;
import models.Product;

@Service
class StockService {
    List<Product> products = List.of();
    boolean restocking = true;
}
