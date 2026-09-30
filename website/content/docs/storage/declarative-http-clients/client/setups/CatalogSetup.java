import java.util.List;
import java.util.Map;

@Setup
class CatalogSetup {
    static Behaviors.Configuration configure(Behaviors.Execution execution) {
        Behaviors.HttpAdapter catalog = (request, running) -> List.of("anvil", "rope");
        return new Behaviors.Configuration(Map.of(), Map.of("catalog", catalog), Map.of(), null);
    }
}
