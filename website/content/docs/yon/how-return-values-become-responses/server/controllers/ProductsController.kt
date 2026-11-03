import com.sun.net.httpserver.HttpExchange

@RequestRoute("/products")
@Public
class ProductsController {
    @GetRoute
    fun getProducts(exchange: HttpExchange) {
        val body = "<main><h1>Products</h1><p>{not-a-template}</p><if>literal</if></main>".toByteArray()
        exchange.responseHeaders.add("content-type", "text/html; charset=utf-8")
        exchange.sendResponseHeaders(200, body.size.toLong())
        exchange.responseBody.write(body)
    }
}
