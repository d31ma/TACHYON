import java.util.Map;

@Setup
class ApplicationSetup {
    static Behaviors.Configuration configure(Behaviors.Execution execution) {
        Behaviors.HttpAdapter manifest = (request, running) -> Map.of("tac", Map.of("api", Map.of("url", "/api")));
        return new Behaviors.Configuration(Map.of(), Map.of("publicConfiguration", manifest), Map.of(), null);
    }
}
