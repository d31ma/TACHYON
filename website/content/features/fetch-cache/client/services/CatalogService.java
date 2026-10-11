import java.util.List;
import java.util.Map;

@Service
class CatalogService {
    @Needs
    private final ProductsClient products;
    int count = 0;

    void load() {
        count = ((List<?>) ((Map<?, ?>) products.list()).get("items")).size();
    }

    void add() {
        products.save(Map.of("name", "anvil"));
    }
}
