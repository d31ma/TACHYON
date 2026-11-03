@Delegate
class CatalogEchoDelegate {
    companion object {
        fun echo(request: Map<String, Any?>): Map<String, Any?> =
            listOf("method", "path", "query", "headers", "body").associateWith { request[it] }
    }
}
