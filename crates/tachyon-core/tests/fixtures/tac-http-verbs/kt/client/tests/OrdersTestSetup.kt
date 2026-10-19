@TestSetup
class OrdersTestSetup {
    companion object {
        fun configure(execution: Behaviors.Execution): Behaviors.Configuration =
            Behaviors.Configuration(http = mapOf("orders" to Behaviors.HttpAdapter { request, _ ->
                val headers = request["headers"] as Map<*, *>
                val body = request["body"] as Map<*, *>?
                mapOf(
                    "method" to request["method"],
                    "path" to request["path"],
                    "header" to (headers["X-Tenant"] ?: headers["X-Reason"]),
                    "item" to (body?.get("item") ?: "none"),
                )
            }))
    }
}
