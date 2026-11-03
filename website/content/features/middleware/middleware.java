import com.sun.net.httpserver.HttpExchange;

class Middleware {
    static void before(HttpExchange exchange) throws java.io.IOException {
        String token = exchange.getRequestHeaders().getFirst("Authorization");
        if (exchange.getRequestURI().getPath().startsWith("/private") && !"let-me-in".equals(token)) {
            byte[] body = "{\"error\":\"Unauthorized\"}".getBytes(java.nio.charset.StandardCharsets.UTF_8);
            exchange.getResponseHeaders().add("content-type", "application/json");
            exchange.sendResponseHeaders(401, body.length);
            exchange.getResponseBody().write(body);
        }
    }

    static void after(HttpExchange exchange) throws java.io.IOException {
        exchange.getResponseHeaders().add("x-served-by", "tachyon");
        exchange.sendResponseHeaders(204, -1);
    }
}
