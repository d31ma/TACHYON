import com.sun.net.httpserver.HttpExchange;
import java.util.List;
import java.util.Map;
import models.Identity;

@IdentityProvider
class AppIdentityProvider {
    private static final Map<String, Identity> SESSIONS = Map.of(
        "ada-token", new Identity("ada", List.of("Admin")),
        "bob-token", new Identity("bob", List.of()));

    Identity identify(HttpExchange exchange) {
        return find(String.valueOf(exchange.getRequestHeaders().getFirst("Authorization")));
    }

    private Identity find(String token) {
        return SESSIONS.get(token);
    }
}
