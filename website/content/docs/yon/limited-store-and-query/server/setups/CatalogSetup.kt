@Setup
class CatalogSetup {
    companion object {
        val ROWS = listOf(mapOf("id" to 1, "status" to "active"), mapOf("id" to 2, "status" to "retired"))

        fun configure(execution: Behaviors.Execution): Behaviors.Configuration {
            val catalog = Behaviors.EntityAdapter { command, _ ->
                val query = command["query"] as Map<*, *>
                val condition = (query["where"] as List<*>)[0] as Map<*, *>
                val keepUnequalRows = condition["operator"] == "ne"
                val matchingRows = ROWS.filter { row ->
                    val isEqual = row[condition["field"]] == condition["value"]
                    if (keepUnequalRows) !isEqual else isEqual
                }
                if (query["kind"] == "count") matchingRows.size.toLong() else matchingRows
            }
            val authorization = Behaviors.Authorization { command, _ -> command["entity"] == "ProductEntity" }
            return Behaviors.Configuration(entities = mapOf("catalog" to catalog), authorizeEntity = authorization)
        }
    }
}
