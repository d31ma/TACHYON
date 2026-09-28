@Setup
class CatalogSetup {
    class MemoryCache : Behaviors.CacheAdapter {
        private val entries = mutableMapOf<String, Behaviors.Entry>()
    
        override fun get(key: String, execution: Behaviors.Execution): Behaviors.Entry {
            return entries[key] ?: Behaviors.Entry()
        }
    
        override fun set(key: String, value: Any?, execution: Behaviors.Write) {
            entries[key] = Behaviors.Entry(true, value, execution.expiresAt)
        }
    
        override fun delete(key: String, execution: Behaviors.Execution) {
            entries.remove(key)
        }
    }

    companion object {
        fun configure(execution: Behaviors.Execution): Behaviors.Configuration = Behaviors.Configuration(
            caches = mapOf("catalog" to MemoryCache()),
            authorize = Behaviors.Authorization { _, _ -> true })
    }
}
