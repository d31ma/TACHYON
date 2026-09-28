@Setup
class CatalogSetup {
    companion object {
        fun configure(execution: Behaviors.Execution): Behaviors.Configuration =
            Behaviors.Configuration(http = mapOf("catalog" to Behaviors.HttpAdapter { _, _ -> mapOf("items" to listOf("anvil")) }))
    }
}
