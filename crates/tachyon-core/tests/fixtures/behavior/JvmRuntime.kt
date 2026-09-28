class RuntimeAdapters {
    companion object {
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
            if (command["operation"] != "query") {
                return command
            }
            val query = command["query"] as Map<*, *>
            return when (query["kind"]) {
                "count" -> 1
                "exists" -> true
                else -> listOf(mapOf("id" to "one", "query" to command["query"]))
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
object RuntimeChecks {
    private var mutationCalls = 0
    private fun fails(code: String, call: () -> Unit) {
        try {
            call()
            error("expected $code")
        } catch (failure: Behaviors.Failure) {
            check(failure.code == code)
        }
    }
    private fun catalogPolicy(maxBytes: Int): Behaviors.Policy = Behaviors.Policy("catalog", 30000, maxBytes, 128)
    private fun cached(context: Behaviors.Context, argument: Any?): Any? {
        return context.invoke("Catalog", "list", arrayOf(argument), intArrayOf(0), catalogPolicy(65536), null) {
            RuntimeAdapters.calls++
            mapOf("id" to RuntimeAdapters.calls)
        }
    }
    private fun mutate(context: Behaviors.Context, put: Boolean, call: () -> Any?): Any? {
        return context.mutate("Catalog", "list", arrayOf("mutation"), intArrayOf(0), catalogPolicy(64), put, null, call = call)
    }
    private fun countMutationCall(): Any? {
        mutationCalls++
        return null
    }
    private fun busyWaitFiveMilliseconds(): Any? {
        val until = System.nanoTime() + 5000000
        while (System.nanoTime() < until) {
            Thread.onSpinWait()
        }
        return null
    }
    private fun mutationChecks() {
        val context = Behaviors.create("RuntimeAdapters", Behaviors.Options("mutation", Behaviors.Signal()))
        for (put in listOf(false, true)) {
            checkMutation(context, put)
        }
        val exchange = Behaviors.Exchange("api", "POST", "/products")
        val exchanged = context.mutate("Catalog", "list", arrayOf("mutation"), intArrayOf(0), catalogPolicy(65536), true, exchange) { mapOf("body" to mapOf("id" to "one")) }
        check(cached(context, "mutation") == exchanged)
        context.close()
        RuntimeAdapters.values.clear()
    }
    private fun checkMutation(context: Behaviors.Context, put: Boolean) {
        RuntimeAdapters.values.clear()
        val previous = cached(context, "mutation")
        mutationCalls = 0
        checkSuccessfulMutation(context, put, previous)
        checkDeniedMutation(context, put)
        checkRejectedMutationResults(context, put)
        checkInterruptedMutations(put)
        checkUnavailableCacheAdapters(context, put)
        checkOversizedMutationKey(context, put)
    }
    private fun checkSuccessfulMutation(context: Behaviors.Context, put: Boolean, previous: Any?) {
        val reads = RuntimeAdapters.reads
        val writes = RuntimeAdapters.writes
        val deletes = RuntimeAdapters.deletes
        val fresh = mutate(context, put) {
            mutationCalls++
            val expectedOperation = if (put) "cache:put" else "cache:evict"
            check(RuntimeAdapters.operation == expectedOperation)
            mapOf("updated" to true)
        }
        check(mutationCalls == 1 && RuntimeAdapters.reads == reads)
        if (put) {
            check(RuntimeAdapters.writes == writes + 1 && RuntimeAdapters.deletes == deletes)
            check(cached(context, "mutation") == fresh)
        } else {
            check(RuntimeAdapters.writes == writes && RuntimeAdapters.deletes == deletes + 1)
            check(cached(context, "mutation") != previous)
        }
    }
    private fun checkDeniedMutation(context: Behaviors.Context, put: Boolean) {
        RuntimeAdapters.allow = false
        fails("authorization") { mutate(context, put) { countMutationCall() } }
        RuntimeAdapters.allow = true
        check(mutationCalls == 1)
    }
    private fun checkRejectedMutationResults(context: Behaviors.Context, put: Boolean) {
        val writes = RuntimeAdapters.writes
        val deletes = RuntimeAdapters.deletes
        try {
            mutate(context, put) { throw IllegalArgumentException("domain") }
            error("missing domain error")
        } catch (expected: IllegalArgumentException) {
            check(expected.message == "domain")
        }
        fails("json-value") { mutate(context, put) { Any() } }
        fails("json-size") { mutate(context, put) { "x".repeat(if (put) 100 else 1048577) } }
        fails("http-request") { context.mutate("Catalog", "list", arrayOf("mutation"), intArrayOf(0), catalogPolicy(65536), put, Behaviors.Exchange("api", "GET", "/products")) { mapOf("body" to "invalid") } }
        check(RuntimeAdapters.writes == writes && RuntimeAdapters.deletes == deletes)
    }
    private fun checkInterruptedMutations(put: Boolean) {
        val writes = RuntimeAdapters.writes
        val deletes = RuntimeAdapters.deletes
        val signal = Behaviors.Signal()
        val aborted = Behaviors.create("RuntimeAdapters", Behaviors.Options("mutation-abort", signal))
        fails("aborted") {
            mutate(aborted, put) {
                signal.abort()
                null
            }
        }
        val timed = Behaviors.create("RuntimeAdapters", Behaviors.Options("mutation-timeout", Behaviors.Signal(), 1))
        fails("timeout") { mutate(timed, put) { busyWaitFiveMilliseconds() } }
        check(RuntimeAdapters.writes == writes && RuntimeAdapters.deletes == deletes)
    }
    private fun checkUnavailableCacheAdapters(context: Behaviors.Context, put: Boolean) {
        RuntimeAdapters.cacheUnavailable = true
        fails("cache-adapter") { mutate(context, put) { countMutationCall() } }
        RuntimeAdapters.cacheUnavailable = false
        check(mutationCalls == 2)
        val withoutCaches = Behaviors.Configuration(emptyMap(), emptyMap(), emptyMap(), Behaviors.Authorization { _, _ -> true })
        val missing = Behaviors.Context(Behaviors.Options("missing", Behaviors.Signal()), withoutCaches)
        fails("cache-adapter") { mutate(missing, put) { countMutationCall() } }
        check(mutationCalls == 2)
    }
    private fun checkOversizedMutationKey(context: Behaviors.Context, put: Boolean) {
        fails("json-size") { context.mutate("Catalog", "list", arrayOf("x".repeat(5000)), intArrayOf(0), catalogPolicy(64), put, null) { countMutationCall() } }
        check(mutationCalls == 2)
    }
    @JvmStatic fun main(arguments: Array<String>) {
        check(arguments.isEmpty())
        val signal = Behaviors.Signal()
        val context = Behaviors.create("RuntimeAdapters", Behaviors.Options("tenant-a", signal))
        checkCachedCalls(context)
        val other = Behaviors.create("RuntimeAdapters", Behaviors.Options("tenant-b", Behaviors.Signal()))
        checkTenantPartition(other)
        checkCacheFailures(context)
        checkPagination()
        checkHttpExchange(context)
        checkRepository(context)
        checkJsonCopies()
        checkTimeout()
        checkCancellation(signal, context, other)
        mutationChecks()
        QueryChecks.run()
        ConfigurationChecks.run()
        ValidationChecks.run()
        println("Kotlin runtime adapter and boundary checks passed")
    }
    private fun checkCachedCalls(context: Behaviors.Context) {
        val first = cached(context, "one")
        check(first == cached(context, "one"))
        check(RuntimeAdapters.calls == 1 && RuntimeAdapters.authorized == 2)
        RuntimeAdapters.allow = false
        fails("authorization") { cached(context, "one") }
        fails("authorization") { context.evict("Catalog", "list", arrayOf("one")) }
        RuntimeAdapters.allow = true
        context.evict("Catalog", "list", arrayOf("one"))
        cached(context, "one")
        check(RuntimeAdapters.calls == 2)
    }
    private fun checkTenantPartition(other: Behaviors.Context) {
        cached(other, "one")
        check(RuntimeAdapters.calls == 3)
        for (key in RuntimeAdapters.values.keys) {
            check(key.contains("module-runtime-test"))
        }
    }
    private fun checkCacheFailures(context: Behaviors.Context) {
        RuntimeAdapters.cacheUnavailable = true
        val callsBeforeOutage = RuntimeAdapters.calls
        fails("cache-adapter") { cached(context, "one") }
        check(RuntimeAdapters.calls == callsBeforeOutage)
        RuntimeAdapters.cacheUnavailable = false
        RuntimeAdapters.values.replaceAll { _, entry -> Behaviors.Entry(true, entry.value, Long.MAX_VALUE) }
        fails("cache-entry") { cached(context, "one") }
        RuntimeAdapters.values.clear()
    }
    private fun checkPagination() {
        val page = Behaviors.page(mapOf("page" to 2, "size" to 5, "sort" to listOf("id")), 20, 100, arrayOf("id"))
        check(page["offset"] == 10L && page["sort"] == listOf("id:asc"))
        fails("pagination") { Behaviors.page(mapOf("sort" to listOf("id", "id")), 20, 100, arrayOf("id")) }
    }
    private fun loadProducts(context: Behaviors.Context, call: () -> Any?): Any? {
        return context.invoke("Client", "load", emptyArray(), intArrayOf(), null, Behaviors.Exchange("api", "GET", "/products"), call = call)
    }
    private fun checkHttpExchange(context: Behaviors.Context) {
        val response = loadProducts(context) { mapOf("query" to mapOf("q" to "name")) }
        check((response as Map<*, *>)["path"] == "/products")
        fails("http-request") { loadProducts(context) { mapOf("headers" to mapOf("X-Test" to "bad\r\nvalue")) } }
        fails("http-request") { loadProducts(context) { mapOf("headers" to mapOf("Host" to "attacker.example")) } }
        fails("http-request") { loadProducts(context) { mapOf("body" to "forbidden") } }
    }
    private fun checkRepository(context: Behaviors.Context) {
        val repository = context.repository("Product")
        check((repository.find() as Map<*, *>)["table"] == "products")
        check((repository.save(mapOf("id" to "one")) as Map<*, *>)["operation"] == "save")
        check((repository.delete("one") as Map<*, *>)["operation"] == "delete")
        fails("entity-id") { repository.save(mapOf("id" to "")) }
    }
    private fun checkJsonCopies() {
        fails("json-number") { Behaviors.copy(Double.NaN, 100) }
        fails("json-number") { Behaviors.copy(9007199254740992L, 100) }
        fails("json-value") { Behaviors.copy(Any(), 100) }
        fails("json-size") { Behaviors.copy("x".repeat(1000), 100) }
        val cyclic = mutableListOf<Any?>()
        cyclic.add(cyclic)
        fails("json-cycle") { Behaviors.copy(cyclic, 100) }
        val source = mutableListOf<Any?>("old")
        val immutable = Behaviors.copy(source, 100)
        source[0] = "new"
        check(immutable == listOf("old"))
    }
    private fun checkTimeout() {
        val timed = Behaviors.create("RuntimeAdapters", Behaviors.Options("timeout", Behaviors.Signal(), 1))
        fails("timeout") { timed.invoke("Slow", "run", emptyArray(), intArrayOf(), null, null) { busyWaitFiveMilliseconds() } }
    }
    private fun checkCancellation(signal: Behaviors.Signal, context: Behaviors.Context, other: Behaviors.Context) {
        signal.abort()
        fails("aborted") { cached(context, "one") }
        fails("aborted") {
            other.operation { execution ->
                other.dispose()
                check(execution.signal.aborted())
                null
            }
        }
        fails("admission") { cached(other, "one") }
    }
}
