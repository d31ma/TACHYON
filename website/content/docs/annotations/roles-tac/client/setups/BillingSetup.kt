@Setup
class BillingSetup {
    companion object {
        fun configure(execution: Behaviors.Execution): Behaviors.Configuration {
            val billing = Behaviors.EntityAdapter { command, _ ->
                val query = command["query"] as Map<*, *>
                if (query["kind"] == "count") 0L else emptyList<Map<String, Any?>>()
            }
            return Behaviors.Configuration(entities = mapOf("publicBilling" to billing),
                authorizeEntity = Behaviors.Authorization { command, _ -> command["entity"] == "BillingEntity" })
        }
    }
}
