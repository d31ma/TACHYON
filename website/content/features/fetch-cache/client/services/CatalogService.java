import java.util.List;
import java.util.Map;

@Service
class CatalogService {
    private final ProductsClient products = new ProductsClient();
    int count = 0;

    void load() {
        count = ((List<?>) ((Map<?, ?>) products.list()).get("items")).size();
    }

    void add() {
        products.save(Map.of("name", "anvil"));
    }
}
