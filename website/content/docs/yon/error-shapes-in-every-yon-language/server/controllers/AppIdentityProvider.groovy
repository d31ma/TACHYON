import com.sun.net.httpserver.HttpExchange
import java.util.List
import java.util.Map
import models.Identity

@IdentityProvider
class AppIdentityProvider {
    private static final Map<String, Identity> SESSIONS = Map.of(
        "ada-token", new Identity("ada", List.of("Admin")),
        "bob-token", new Identity("bob", List.of()))
    private static final String BROKEN_TOKEN = "broken-token"

    Identity identify(HttpExchange exchange) {
        String token = String.valueOf(exchange.getRequestHeaders().getFirst("Authorization"))
        refuseBroken(token)
        return find(token)
    }

    private void refuseBroken(String token) {
        if (token.equals(BROKEN_TOKEN)) throw new IllegalStateException("the session store is down")
    }

    private Identity find(String token) {
        return SESSIONS.get(token)
    }
}
