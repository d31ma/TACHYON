@Setup
class CatalogSetup {
    companion object {
        fun configure(execution: Behaviors.Execution): Behaviors.Configuration {
            val catalog = Behaviors.EntityAdapter { command, _ ->
                val query = command["query"] as Map<*, *>
                if (query["kind"] == "count") 0L else emptyList<Map<String, Any?>>()
            }
            return Behaviors.Configuration(entities = mapOf("publicCatalog" to catalog),
                authorizeEntity = Behaviors.Authorization { command, _ -> command["entity"] == "ProductEntity" })
        }
    }
}
