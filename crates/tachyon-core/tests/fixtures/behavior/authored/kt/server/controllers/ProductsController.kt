@RequestRoute("/products")
@Public
class ProductsController {
    @GetRoute
    fun getProducts(exchange: com.sun.net.httpserver.HttpExchange): Any? {
        check(exchange.requestMethod == "GET")
        return ProductChecksHelper().run()
    }
}
