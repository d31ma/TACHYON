@TestSetup
class OrdersTestSetup {
    companion object {
        fun configure(execution: Behaviors.Execution): Behaviors.Configuration =
            Behaviors.Configuration(http = mapOf("orders" to Behaviors.HttpAdapter { request, _ ->
                if (request["path"] == "/api/claims/a-7") throw Behaviors.Failure("HTTP_STATUS", null, "private fake text", 409, mapOf("reason" to "taken", "holder" to "south"))
                val headers = request["headers"] as Map<*, *>
                val body = request["body"] as Map<*, *>?
                mapOf(
                    "method" to request["method"],
                    "path" to request["path"],
                    "header" to (headers["X-Tenant"] ?: headers["X-Reason"]),
                    "item" to (body?.get("item") ?: "none"),
                    "param" to ((request["query"] as? Map<*, *>)?.get("soft") ?: "none"),
                )
            }))
    }
}
