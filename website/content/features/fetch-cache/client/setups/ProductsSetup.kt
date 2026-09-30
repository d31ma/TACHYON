@Setup
class ProductsSetup {
    companion object {
        fun configure(execution: Behaviors.Execution): Behaviors.Configuration =
            Behaviors.Configuration(http = mapOf("products" to Behaviors.HttpAdapter { _, _ -> mapOf("items" to listOf("anvil", "hammer")) }))
    }
}
