private fun behaviorBuiltinCache(name: String): Behaviors.CacheAdapter? = TacBrowserCache(name)

private fun behaviorBuiltinEntities(name: String): Behaviors.EntityAdapter? =
    if (name == TacBrowserStorage.INDEXED_DB) TacBrowserEntities else null

private object TacBrowserStorage {
    const val INDEXED_DB = "indexeddb"
    private const val NO_ANSWER = "Tachyon storage did not answer."
    private const val REFUSED = "Tachyon storage refused the request."
    private const val NOT_LOADED = "storage_not_loaded"

    fun ask(request: Map<String, Any?>): Any? {
        val answer = tacParseJson(tacHostStorage(Behaviors.json(request))) as? Map<*, *>
            ?: throw Behaviors.Failure("storage", null, NO_ANSWER)
        val error = answer["error"]
        if (error != null) throw Behaviors.Failure(refusal(answer["code"]), error.toString(), REFUSED)
        return answer["value"]
    }

    private fun refusal(code: Any?): String = if (code == NOT_LOADED) NOT_LOADED else "storage"
}

private class TacBrowserCache(private val name: String) : Behaviors.CacheAdapter {
    override fun get(key: String, execution: Behaviors.Execution): Behaviors.Entry {
        val entry = TacBrowserStorage.ask(mapOf("op" to "cache.get", "cache" to name, "key" to key)) as? Map<*, *>
        if (entry == null || entry["hit"] != true) return Behaviors.Entry()
        val expiresAt = (entry["expiresAt"] as? Number)?.toLong() ?: 0L
        return Behaviors.Entry(true, entry["value"], expiresAt)
    }

    override fun set(key: String, value: Any?, execution: Behaviors.Write) {
        TacBrowserStorage.ask(
            mapOf(
                "op" to "cache.set", "cache" to name, "key" to key, "value" to value,
                "expiresAt" to execution.expiresAt, "maxEntries" to execution.policy.maxEntries,
            )
        )
    }

    override fun delete(key: String, execution: Behaviors.Execution) {
        TacBrowserStorage.ask(mapOf("op" to "cache.delete", "cache" to name, "key" to key))
    }
}

private object TacBrowserEntities : Behaviors.EntityAdapter {
    override fun execute(command: Map<String, Any?>, execution: Behaviors.Execution): Any? =
        TacBrowserStorage.ask(mapOf("op" to "entity", "command" to command))
}
