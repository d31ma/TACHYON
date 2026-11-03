import java.util.List;
import java.util.Map;

@Setup
class ProductsSetup {
    static Behaviors.Configuration configure(Behaviors.Execution execution) {
        Behaviors.HttpAdapter products = (request, running) -> Map.of("items", List.of("anvil", "hammer"));
        return new Behaviors.Configuration(Map.of(), Map.of("products", products), Map.of(), null);
    }
}
