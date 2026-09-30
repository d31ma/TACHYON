import java.util.Map;

@Service
class CatalogService {
    Behaviors.Context behaviors;
    long offset = 0;

    Object page(@PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]}) Map<String, Object> input) {
        return input.get("offset");
    }

    @OnStart
    void initialize() {
        behaviors = Behaviors.create("ApplicationSetup", new Behaviors.Options("public-catalog", new Behaviors.Signal()));
        offset = ((Number) page(Map.of("page", 1))).longValue();
    }
}
