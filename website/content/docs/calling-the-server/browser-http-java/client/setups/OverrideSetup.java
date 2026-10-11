import java.util.Map;

@Setup
class OverrideSetup {
    static Behaviors.Configuration configure(Behaviors.Execution execution) {
        Behaviors.HttpAdapter override = (request, running) -> Map.of("message", "Custom adapter wins");
        return new Behaviors.Configuration(Map.of(), Map.of("override", override), Map.of(), null);
    }
}
