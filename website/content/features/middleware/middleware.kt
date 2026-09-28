import com.sun.net.httpserver.HttpExchange

object Middleware {
    fun before(exchange: HttpExchange) {
        val token = exchange.requestHeaders.getFirst("Authorization")
        if (exchange.requestURI.path.startsWith("/private") && token != "let-me-in") {
            val body = "{\"error\":\"Unauthorized\"}".toByteArray()
            exchange.responseHeaders.add("content-type", "application/json")
            exchange.sendResponseHeaders(401, body.size.toLong())
            exchange.responseBody.write(body)
        }
    }

    fun after(exchange: HttpExchange) {
        exchange.responseHeaders.add("x-served-by", "tachyon")
        exchange.sendResponseHeaders(204, -1)
    }
}
