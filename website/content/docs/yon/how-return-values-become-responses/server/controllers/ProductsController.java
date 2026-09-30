import com.sun.net.httpserver.HttpExchange;
import java.nio.charset.StandardCharsets;

@RequestRoute("/products")
@Public
class ProductsController {
    @GetRoute
    void getProducts(HttpExchange exchange) throws java.io.IOException {
        byte[] body = "<main><h1>Products</h1><p>{not-a-template}</p><if>literal</if></main>".getBytes(StandardCharsets.UTF_8);
        exchange.getResponseHeaders().add("content-type", "text/html; charset=utf-8");
        exchange.sendResponseHeaders(200, body.length);
        exchange.getResponseBody().write(body);
    }
}
