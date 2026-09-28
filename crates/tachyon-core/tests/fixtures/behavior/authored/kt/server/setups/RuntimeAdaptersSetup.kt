@Setup()
class RuntimeAdaptersSetup {
    companion object {
        private const val QUERY = "query"
        val values = mutableMapOf<String, Behaviors.Entry>()
        var authorized = 0
        var calls = 0
        var reads = 0
        var writes = 0
        var deletes = 0
        var operation = ""
        var configured = 0
        var allow = true
        var cacheUnavailable = false
        fun configure(execution: Behaviors.Execution): Behaviors.Configuration {
            configured++
            execution.check()
            return Behaviors.Configuration(
                caches = mapOf("catalog" to catalogCache()),
                http = mapOf("api" to Behaviors.HttpAdapter { request, current -> checkHttpRequest(request, current) }),
                entities = mapOf("data" to Behaviors.EntityAdapter { command, current -> runEntityCommand(command, current) }),
                authorize = Behaviors.Authorization { call, current -> authorizeCache(call, current) },
                authorizeEntity = Behaviors.Authorization { _, current -> authorizeEntity(current) }
            )
        }
        private fun catalogCache(): Behaviors.CacheAdapter = object : Behaviors.CacheAdapter {
            override fun get(key: String, execution: Behaviors.Execution): Behaviors.Entry {
                execution.check()
                reads++
                failWhenCacheUnavailable()
                return values[key] ?: Behaviors.Entry()
            }
            override fun set(key: String, value: Any?, execution: Behaviors.Write) {
                execution.execution.check()
                failWhenCacheUnavailable()
                writes++
                values[key] = Behaviors.Entry(true, value, execution.expiresAt)
            }
            override fun delete(key: String, execution: Behaviors.Execution) {
                execution.check()
                failWhenCacheUnavailable()
                deletes++
                values.remove(key)
            }
        }
        private fun failWhenCacheUnavailable() {
            if (cacheUnavailable) {
                error("private adapter error")
            }
        }
        private fun checkHttpRequest(request: Map<String, Any?>, current: Behaviors.Execution): Any? {
            current.check()
            check(request["redirect"] == "error" && request["credentials"] == "omit")
            return request
        }
        private fun runEntityCommand(command: Map<String, Any?>, current: Behaviors.Execution): Any? {
            current.check()
            if (command["operation"] != QUERY) {
                return command
            }
            val query = command[QUERY] as Map<*, *>
            return when (query["kind"]) {
                "count" -> 1
                "exists" -> true
                else -> listOf(mapOf("id" to "one", QUERY to command[QUERY]))
            }
        }
        private fun authorizeCache(call: Map<String, Any?>, current: Behaviors.Execution): Boolean {
            current.check()
            operation = call["operation"] as String
            check(operation in setOf("cache:read", "cache:evict", "cache:put"))
            authorized++
            return allow
        }
        private fun authorizeEntity(current: Behaviors.Execution): Boolean {
            current.check()
            return allow
        }
    }
}

