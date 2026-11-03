import java.util.ArrayList;
import java.util.List;
import java.util.Map;

@Setup
class UsersSetup {
    static Behaviors.Configuration configure(Behaviors.Execution execution) {
        List<Object> saved = new ArrayList<>();
        Behaviors.HttpAdapter users = (request, running) -> {
            if ("GET".equals(request.get("method"))) return new ArrayList<>(saved);
            Object name = request.get("body");
            saved.add(name);
            return Map.of("name", name);
        };
        return new Behaviors.Configuration(Map.of(), Map.of("users", users), Map.of(), null);
    }
}
