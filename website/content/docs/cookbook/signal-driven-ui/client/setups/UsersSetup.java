import java.util.Map;

@Setup
class UsersSetup {
    static Behaviors.Configuration configure(Behaviors.Execution execution) {
        Behaviors.HttpAdapter users = (request, running) -> Map.of("name", request.get("body"));
        return new Behaviors.Configuration(Map.of(), Map.of("users", users), Map.of(), null);
    }
}
