import java.util.Map;

@TestSetup
class HelloTestSetup {
    static Behaviors.Configuration configure(Behaviors.Execution execution) {
        Behaviors.HttpAdapter greeting = (request, running) -> Map.of("message", "Hi");
        return new Behaviors.Configuration(Map.of(), Map.of("greeting", greeting), Map.of(), null);
    }
}
