@Setup
class CatalogSetup {
    companion object {
        fun configure(execution: Behaviors.Execution): Behaviors.Configuration = Behaviors.Configuration(
            caches = mapOf("catalog" to MemoryCacheDelegate()),
            authorize = Behaviors.Authorization { _, _ -> true })
    }
}
