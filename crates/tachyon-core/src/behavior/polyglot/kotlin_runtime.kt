object Behaviors {
    private const val MODULE = "__MODULE__"
    private const val ENVIRONMENT = "__ENVIRONMENT__"
    class Failure(val code: String, val path: String? = null, message: String = code) : RuntimeException(message)
    private fun fail(code: String): Nothing = throw Failure(code)
    class Signal internal constructor(private val parent: Signal? = null, private val owner: Context? = null) {
        @Volatile private var cancelled = false
        fun abort() { cancelled = true }
        fun aborted(): Boolean = cancelled || parent?.aborted() == true || owner?.closed == true
    }
    class Options(val partition: String, val signal: Signal, val timeoutMs: Long = 10000, profiles: List<String> = emptyList(), properties: Map<String, Any?> = emptyMap<String, Any?>()) {
        val profiles: List<String>
        val properties: Map<String, String>
        init {
            if (partition.isBlank() || bytes(partition) > 256 || timeoutMs !in 1..60000 || profiles.size > 32 || profiles.any { !it.matches(Regex("[A-Za-z_\$][A-Za-z0-9_\$]{0,63}")) } || profiles.toSet().size != profiles.size) fail("configuration")
            this.profiles = java.util.Collections.unmodifiableList(profiles.toList()); this.properties = propertyInputs(properties)
        }
    }
    class Execution internal constructor(options: Options, private val owner: Context?) {
        val environment = ENVIRONMENT
        val partition = options.partition
        val signal = Signal(options.signal, owner)
        val deadline = System.currentTimeMillis() + options.timeoutMs
        internal val monotonicDeadline = System.nanoTime() + options.timeoutMs * 1000000
        fun check() {
            if (signal.aborted() || owner?.closed == true) fail("aborted")
            if (System.nanoTime() - monotonicDeadline >= 0) fail("timeout")
        }
    }
    interface CacheAdapter {
        fun get(key: String, execution: Execution): Entry
        fun set(key: String, value: Any?, execution: Write)
        fun delete(key: String, execution: Execution)
    }
    fun interface HttpAdapter { fun exchange(request: Map<String, Any?>, execution: Execution): Any? }
    fun interface EntityAdapter { fun execute(command: Map<String, Any?>, execution: Execution): Any? }
    fun interface Authorization { fun allow(call: Map<String, Any?>, execution: Execution): Boolean }
    class Configuration(
        caches: Map<String, CacheAdapter> = emptyMap(),
        http: Map<String, HttpAdapter> = emptyMap(),
        entities: Map<String, EntityAdapter> = emptyMap(),
        val authorize: Authorization? = null,
        val authorizeEntity: Authorization? = null
    ) {
        val caches: Map<String, CacheAdapter> = java.util.Collections.unmodifiableMap(caches.toMap())
        val http: Map<String, HttpAdapter> = java.util.Collections.unmodifiableMap(http.toMap())
        val entities: Map<String, EntityAdapter> = java.util.Collections.unmodifiableMap(entities.toMap())
    }
    class Entry(val hit: Boolean = false, val value: Any? = null, val expiresAt: Long = 0)
    class Policy(val name: String, val ttlMs: Long, val maxBytes: Int, val maxEntries: Int) {
        init { if (name.isEmpty() || bytes(name) > 128 || ttlMs !in 1..86400000 || maxBytes !in 1..1048576 || maxEntries !in 1..10000) fail("cache-policy") }
    }
    class Write(val execution: Execution, val policy: Policy, val expiresAt: Long)
    class Exchange(
        val client: String, val method: String, val path: String,
        val query: List<Pair<String, Int>> = emptyList(),
        val headers: List<Pair<String, Int>> = emptyList(),
        val body: Int = -1,
        val pathVariables: List<Pair<String, Int>> = emptyList(),
    ) {
        fun declared(): Boolean = query.isNotEmpty() || headers.isNotEmpty() || body >= 0 || pathVariables.isNotEmpty()
        fun filled(arguments: Array<Any?>): Exchange {
            if (pathVariables.isEmpty()) return this
            var filled = path
            for ((name, index) in pathVariables) {
                val value = arguments.getOrNull(index)
                val text = if (value is String || value is Number || value is Boolean) value.toString() else null
                if (text == null || !Regex("[A-Za-z0-9._~-]{1,256}").matches(text) || text == "." || text == "..") fail("http-request")
                filled = filled.replace("{$name}", text)
            }
            return Exchange(client, method, filled, query, headers, body, pathVariables)
        }
    }

    private fun exchangeRequest(metadata: Exchange, arguments: Array<Any?>): Any? {
        fun pick(index: Int): Any? {
            if (index < 0 || index >= arguments.size) fail("http-request")
            return arguments[index]
        }
        val descriptor = mutableMapOf<String, Any?>(
            "query" to metadata.query.associate { it.first to pick(it.second) },
            "headers" to metadata.headers.associate { it.first to pick(it.second) },
        )
        if (metadata.body >= 0) descriptor["body"] = pick(metadata.body)
        return descriptor
    }
    class Entity(val name: String, val adapter: String, val table: String, val id: String)
    fun create(name: String, options: Options): Context {
        val execution = Execution(options, null)
        execution.check()
        if (!configurationActive(name, options)) fail("inactive-configuration")
        val setupKey = name + options.profiles.sorted() + options.properties.toList().sortedBy { it.first }
        return Context(options, acquireSetup(setupKey, name, execution), setupKey)
    }
    private class SharedSetup(val configuration: Configuration) { var users = 1 }
    private val setups = HashMap<String, SharedSetup>()
    private fun acquireSetup(setupKey: String, name: String, execution: Execution): Configuration {
        synchronized(setups) {
            val shared = setups[setupKey]
            if (shared != null) {
                shared.users++
                return shared.configuration
            }
            val configuration = try { configure(name, execution) } catch (error: Failure) { throw error } catch (_: RuntimeException) { fail("configuration") }
            execution.check()
            setups[setupKey] = SharedSetup(configuration)
            return configuration
        }
    }
    internal fun releaseSetup(setupKey: String) {
        synchronized(setups) {
            val shared = setups[setupKey] ?: return
            shared.users--
            if (shared.users == 0) setups.remove(setupKey)
        }
    }
    private fun configure(name: String, execution: Execution): Configuration = when (name) {
        __CONFIGURATIONS__
        else -> fail("configuration")
    }
    private fun configurationActive(name: String, options: Options): Boolean = when (name) {
        __SELECTIONS__
        else -> true
    }
    private fun active(options: Options, profiles: Array<String>, property: String?, expected: String?, matchIfMissing: Boolean): Boolean =
        (profiles.isEmpty() || profiles.any { it in options.profiles }) && (property == null || (!options.properties.keys.any { it.startsWith(property + "_") } && if (options.properties.containsKey(property)) options.properties[property] == expected else matchIfMissing))
    class Property(val field: String, val name: String, val type: String, val fallback: Any?, val hasDefault: Boolean, val minimum: Number?, val maximum: Number?)
    private fun propertyInputs(supplied: Map<String, Any?>): Map<String, String> {
        val values = if ("tac" in supplied || "yon" in supplied) applicationInputs(supplied).filterKeys { it.startsWith(ENVIRONMENT + ".") }.mapKeys { it.key.replace('.', '_').uppercase(java.util.Locale.ROOT) } else supplied
        if (values.size > 256) fail("configuration")
        val result = linkedMapOf<String, String>()
        for ((key, value) in values as Map<*, *>) {
            if (key !is String || value !is String || key.length > 256 || !key.matches(Regex("[A-Z][A-Z0-9_]*")) || !key.startsWith(if (ENVIRONMENT == "yon") "YON_" else "TAC_") || bytes(value) > 4096) fail("configuration")
            result[key] = value
        }
        try { copy(result, 65536) } catch (_: RuntimeException) { fail("configuration") }
        return java.util.Collections.unmodifiableMap(result)
    }
    fun properties(name: String, values: Map<String, Any?>): Map<String, Any?> = when (name) {
        __PROPERTIES__
        else -> fail("configuration-properties")
    }
    fun values(name: String, application: Map<*, *>): Map<String, Any?> = when (name) {
        __VALUES__
        else -> fail("configuration-properties")
    }
    private fun applicationInputs(application: Map<*, *>): Map<String, String> {
        try {
            val root = map(copy(application, 65536))
            val flat = linkedMapOf<String, String>()
            if ("tac" !in root && "yon" !in root) fail("configuration-properties")
            for ((key, value) in root) {
                if (key != "tac" && key != "yon") continue
                flattenApplication(map(value), key, 0, flat)
            }
            return flat
        } catch (_: RuntimeException) { fail("configuration-properties") }
    }
    private fun flattenApplication(value: Any?, path: String, depth: Int, flat: MutableMap<String, String>) {
        if (depth > 16 || path.length > 256 || flat.size >= 256) fail("configuration-properties")
        if (path == "yon.server.port" && (value !is Number || value.toDouble() != Math.rint(value.toDouble()))) fail("configuration-properties")
        if (value is Map<*, *>) {
            if (depth > 0 && value.isEmpty()) flat[path + "."] = ""
            for ((key, child) in map(value)) {
                if (!key.matches(Regex("[a-z][a-z0-9]*"))) fail("configuration-properties")
                flattenApplication(child, path + "." + key, depth + 1, flat)
            }
        } else {
            if (value == null || value is List<*>) { flat[path + "."] = ""; return }
            if (value !is String && value !is Boolean && value !is Number) fail("configuration-properties")
            val text = if (value is Number && value.toDouble() == Math.rint(value.toDouble())) value.toLong().toString() else value.toString()
            if (bytes(text) > 4096 || flat.size >= 256) fail("configuration-properties")
            flat[path] = text
        }
    }
    fun bindValues(fields: Array<Property>, application: Map<*, *>): Map<String, Any?> {
        val values = applicationInputs(application)
        val result = linkedMapOf<String, Any?>()
        for (field in fields) {
            if (values.keys.any { it.startsWith(field.name + ".") }) fail("configuration-properties")
            val value = propertyValue(field, values[field.name])
            if (value is Number) {
                val numeric = value.toDouble()
                if (!numeric.isFinite() || (field.minimum != null && numeric < field.minimum.toDouble()) || (field.maximum != null && numeric > field.maximum.toDouble())) fail("configuration-properties")
            }
            result[field.field] = value
        }
        return try { map(copy(result, 65536)) } catch (_: RuntimeException) { fail("configuration-properties") }
    }
    fun bind(prefix: String, fields: Array<Property>, supplied: Map<String, Any?>): Map<String, Any?> {
        val values = try { propertyInputs(supplied) } catch (_: RuntimeException) { fail("configuration-properties") }
        val known = fields.map { prefix + it.name }.toSet()
        if (values.keys.any { it.startsWith(prefix) && it !in known }) fail("configuration-properties")
        val result = linkedMapOf<String, Any?>()
        for (field in fields) {
            val value = propertyValue(field, values[prefix + field.name])
            if (value is Number) {
                val numeric = value.toDouble()
                if (!numeric.isFinite() || (field.minimum != null && numeric < field.minimum.toDouble()) || (field.maximum != null && numeric > field.maximum.toDouble())) fail("configuration-properties")
            }
            result[field.field] = value
        }
        return try { map(copy(result, 65536)) } catch (_: RuntimeException) { fail("configuration-properties") }
    }
    private fun propertyValue(field: Property, raw: String?): Any? {
        val value = if (raw != null) parseProperty(raw, field.type)
            else { if (!field.hasDefault) fail("configuration-properties"); field.fallback }
        if (field.type == "string" && (value !is String || bytes(value) > 4096)) fail("configuration-properties")
        return value
    }
    private fun parseProperty(value: String, type: String): Any {
        if (type == "string") return value
        if (type == "boolean") { if (value != "true" && value != "false") fail("configuration-properties"); return value == "true" }
        try {
            if (type == "integer" && value.matches(Regex("-?(0|[1-9][0-9]*)"))) {
                val result = value.toLong(); if (result !in -9007199254740991L..9007199254740991L) fail("configuration-properties"); return result
            }
            if (type == "number" && value.matches(Regex("-?(0|[1-9][0-9]*)(\\.[0-9]+)?([eE][+-]?[0-9]+)?"))) {
                val result = value.toDouble(); if (!result.isFinite() || kotlin.math.abs(result) > 9007199254740991.0) fail("configuration-properties"); return result
            }
        } catch (_: RuntimeException) { fail("configuration-properties") }
        fail("configuration-properties")
    }
    class Rule(val field: String, val notNull: Boolean, val notBlank: Boolean, val sizeMin: Long?, val sizeMax: Long?, val minimum: Number?, val maximum: Number?)
    fun validate(name: String, value: Any?): Map<String, Any?> = when (name) {
        __VALIDATIONS__
        else -> throw Failure("validation", "$.schema")
    }
    fun validateRules(input: Any?, rules: Array<Rule>): Map<String, Any?> {
        val copied = try { copy(input, 1048576) } catch (_: RuntimeException) { throw Failure("validation", "$.json") }
        if (copied !is Map<*, *>) throw Failure("validation", "$.object")
        val value = map(copied)
        for (rule in rules) {
            val field = value[rule.field]
            if (rule.notNull && field == null) throw Failure("validation", rule.field + ".notNull")
            if (rule.notBlank && (field !is String || field.codePoints().allMatch { whiteSpace(it) })) throw Failure("validation", rule.field + ".notBlank")
            if (field == null) continue
            if (rule.sizeMin != null || rule.sizeMax != null) {
                val size = when (field) {
                    is String -> bytes(field).toLong()
                    is List<*> -> field.size.toLong()
                    is Map<*, *> -> field.size.toLong()
                    else -> throw Failure("validation", rule.field + ".size")
                }
                if ((rule.sizeMin != null && size < rule.sizeMin) || (rule.sizeMax != null && size > rule.sizeMax)) throw Failure("validation", rule.field + ".size")
            }
            if (rule.minimum != null && (field !is Number || field.toDouble() < rule.minimum.toDouble())) throw Failure("validation", rule.field + ".min")
            if (rule.maximum != null && (field !is Number || field.toDouble() > rule.maximum.toDouble())) throw Failure("validation", rule.field + ".max")
        }
        return value
    }
    private fun whiteSpace(code: Int): Boolean = code in 9..13 || code == 32 || code == 133 || code == 160 || code == 5760 || code in 8192..8202 || code == 8232 || code == 8233 || code == 8239 || code == 8287 || code == 12288
    private fun entity(name: String): Entity = when (name) {
        __ENTITIES__
        else -> fail("entity")
    }
    class Retry(val maxRetries: Int, val delayMs: Int, val multiplier: Double, val maxDelayMs: Int, val jitterMs: Int)
    class Limit(val limit: Int, val blocking: Boolean)
    class Transaction(val adapter: String)
    object OpenTransactions {
        private val open: ThreadLocal<Map<String, String>> = ThreadLocal.withInitial { emptyMap() }
        fun of(adapter: String): String? = open.get()[adapter]
        fun within(adapter: String, identifier: String, call: () -> Any?): Any? {
            val outer = open.get()
            open.set(outer + (adapter to identifier))
            try { return call() } finally { open.set(outer) }
        }
    }
    private fun retryable(error: RuntimeException): Boolean = error !is Failure || error.code.endsWith("-adapter")
    class Context internal constructor(private val options: Options, internal val configuration: Configuration, private val setupKey: String) : AutoCloseable {
        @Volatile internal var closed = false
        private var active = 0
        private val slots = java.util.concurrent.ConcurrentHashMap<String, java.util.concurrent.Semaphore>()
        private fun transacted(policy: Transaction?, execution: Execution, call: () -> Any?): Any? {
            if (policy == null || OpenTransactions.of(policy.adapter) != null) return call()
            val adapter = entityAdapter(policy.adapter)
            val begun = control(adapter, execution, "begin", null)
            val identifier = (begun as? Map<*, *>)?.get("transaction")
            if (identifier !is String || identifier.isEmpty() || identifier.toByteArray().size > 128) fail("entity-adapter")
            val value: Any?
            try { value = OpenTransactions.within(policy.adapter, identifier, call) }
            catch (error: RuntimeException) {
                try { control(adapter, execution, "rollback", identifier) } catch (ignored: RuntimeException) { }
                throw error
            }
            control(adapter, execution, "commit", identifier)
            return value
        }
        private fun control(adapter: EntityAdapter, execution: Execution, action: String, identifier: String?): Any? {
            val command = if (identifier == null) mapOf("operation" to "transaction", "action" to action)
                else mapOf("operation" to "transaction", "action" to action, "transaction" to identifier)
            val result = try { adapter.execute(command, execution) }
                catch (error: Failure) { throw error } catch (_: RuntimeException) { fail("entity-adapter") }
            execution.check()
            return result
        }
        override fun close() { dispose() }
        fun dispose() {
            synchronized(this) { if (closed) return; closed = true }
            releaseSetup(setupKey)
        }
        internal fun operation(call: (Execution) -> Any?): Any? {
            synchronized(this) { if (closed || active >= 64) fail("admission"); active++ }
            val execution = Execution(options, this)
            try { execution.check(); val value = call(execution); execution.check(); return value }
            finally { synchronized(this) { active-- } }
        }
        internal fun resilient(type: String, method: String, retry: Retry?, limit: Limit?, transactional: Transaction?, execution: Execution, call: () -> Any?): Any? {
            val body = { transacted(transactional, execution, call) }
            if (limit == null) return attempt(retry, execution, body)
            val slot = slots.computeIfAbsent("$type.$method") { java.util.concurrent.Semaphore(limit.limit, true) }
            val acquired = try {
                if (limit.blocking) slot.tryAcquire(maxOf(0L, execution.monotonicDeadline - System.nanoTime()), java.util.concurrent.TimeUnit.NANOSECONDS)
                else slot.tryAcquire()
            } catch (_: InterruptedException) { Thread.currentThread().interrupt(); fail("aborted") }
            if (!acquired) fail(if (limit.blocking) "timeout" else "capacity")
            try { return attempt(retry, execution, body) } finally { slot.release() }
        }
        private fun attempt(retry: Retry?, execution: Execution, call: () -> Any?): Any? {
            if (retry == null) return call()
            var attempt = 0
            while (true) {
                try { return call() }
                catch (error: RuntimeException) {
                    if (attempt >= retry.maxRetries || !retryable(error)) throw error
                    execution.check()
                    val wait = Math.round(minOf(retry.delayMs * Math.pow(retry.multiplier, attempt.toDouble()), retry.maxDelayMs.toDouble()) + Math.random() * retry.jitterMs)
                    if (wait > 0) {
                        try { Thread.sleep(wait) } catch (_: InterruptedException) { Thread.currentThread().interrupt(); fail("aborted") }
                    }
                    execution.check()
                    attempt++
                }
            }
        }
        fun invoke(type: String, method: String, arguments: Array<Any?>, indices: IntArray, policy: Policy?, declaredExchange: Exchange?, retry: Retry? = null, limit: Limit? = null, transactional: Transaction? = null, call: () -> Any?): Any? = operation { execution ->
            val exchange = declaredExchange?.filled(arguments)
            val key = if (policy != null) cacheKey(type, method, arguments, indices, policy, execution, "cache:read") else null
            val cache = if (policy != null) cacheAdapter(policy.name) else null
            if (cache != null && key != null && policy != null) {
                val entry = try { cache.get(key, execution) } catch (_: RuntimeException) { fail("cache-adapter") }
                execution.check()
                val now = System.currentTimeMillis()
                if (entry.hit && entry.expiresAt > now + policy.ttlMs) fail("cache-entry")
                if (entry.hit && entry.expiresAt > now) return@operation copy(entry.value, policy.maxBytes)
            }
            var value = if (exchange != null && exchange.declared()) exchangeRequest(exchange, arguments)
                else resilient(type, method, retry, limit, transactional, execution, call)
            execution.check()
            if (exchange != null) value = exchange(value, exchange, execution)
            value = copy(value, policy?.maxBytes ?: 1048576)
            if (cache != null && key != null && policy != null) {
                try { cache.set(key, copy(value, policy.maxBytes), Write(execution, policy, System.currentTimeMillis() + policy.ttlMs)) }
                catch (_: RuntimeException) { fail("cache-adapter") }
            }
            execution.check()
            value
        }
        fun mutate(type: String, target: String, arguments: Array<Any?>, indices: IntArray, policy: Policy, put: Boolean, declaredExchange: Exchange?, retry: Retry? = null, limit: Limit? = null, transactional: Transaction? = null, call: () -> Any?): Any? = operation { execution ->
            val exchange = declaredExchange?.filled(arguments)
            val key = cacheKey(type, target, arguments, indices, policy, execution, if (put) "cache:put" else "cache:evict")
            val cache = cacheAdapter(policy.name)
            var value = resilient(type, target, retry, limit, transactional, execution, call)
            execution.check()
            if (exchange != null) value = exchange(value, exchange, execution)
            value = copy(value, if (put) policy.maxBytes else 1048576)
            execution.check()
            try {
                if (put) cache.set(key, value, Write(execution, policy, System.currentTimeMillis() + policy.ttlMs))
                else cache.delete(key, execution)
            } catch (_: RuntimeException) { fail("cache-adapter") }
            execution.check()
            value
        }
        private fun cacheKey(type: String, method: String, arguments: Array<Any?>, indices: IntArray, policy: Policy, execution: Execution, operation: String): String {
            val selected = indices.map { if (it !in arguments.indices) fail("cache-key"); copy(arguments[it], 4096) }
            val immutable = copy(selected, 4096)
            allowCache(record("operation" to operation, "cache" to policy.name, "type" to type, "method" to method, "keyArguments" to immutable), policy, execution)
            return encode(copy(listOf(options.partition, policy.name, MODULE, type, method, immutable), 4096))
        }
        private fun allowCache(call: Map<String, Any?>, policy: Policy, execution: Execution) {
            if (configuration.authorize == null && builtinCache(policy.name)) return
            val authorize = configuration.authorize ?: fail("authorization")
            val allowed = try { authorize.allow(call, execution) } catch (_: RuntimeException) { fail("authorization") }
            execution.check()
            if (!allowed) fail("authorization")
        }
        private fun allowEntity(command: Map<String, Any?>, entity: Entity, execution: Execution) {
            if (configuration.authorizeEntity == null && builtinEntities(entity.adapter)) return
            val authorize = configuration.authorizeEntity ?: fail("authorization")
            val allowed = try { authorize.allow(command, execution) } catch (_: RuntimeException) { fail("authorization") }
            execution.check()
            if (!allowed) fail("authorization")
        }
        private fun cacheAdapter(name: String): CacheAdapter = configuration.caches[name] ?: behaviorBuiltinCache(name) ?: fail("cache-adapter")
        private fun entityAdapter(name: String): EntityAdapter = configuration.entities[name] ?: behaviorBuiltinEntities(name) ?: fail("entity-adapter")
        private fun builtinCache(name: String): Boolean = name !in configuration.caches && behaviorBuiltinCache(name) != null
        private fun builtinEntities(name: String): Boolean = name !in configuration.entities && behaviorBuiltinEntities(name) != null
        fun evict(type: String, method: String, arguments: Array<Any?>) {
            operation { execution ->
                __EVICTIONS__
                fail("cache-method")
            }
        }
        fun repository(name: String): Repository = Repository(this, entity(name))
        fun query(entity: Entity, kind: String, fields: Array<String>, where: Array<Array<Any?>>, orderBy: Array<Array<String>>, input: Any?, defaultSize: Int, maxSize: Int): Any? = operation { execution ->
            val query = queryDescriptor(kind, fields, where, orderBy, input, defaultSize, maxSize)
            val joined = OpenTransactions.of(entity.adapter)
            val command = map(copy(if (joined == null) record("operation" to "query", "entity" to entity.name, "table" to entity.table, "idField" to entity.id, "query" to query)
                else record("operation" to "query", "entity" to entity.name, "table" to entity.table, "idField" to entity.id, "query" to query, "transaction" to joined), 1048576))
            allowEntity(command, entity, execution)
            val adapter = entityAdapter(entity.adapter)
            val raw = try { adapter.execute(command, execution) } catch (_: RuntimeException) { fail("entity-adapter") }
            execution.check()
            val result = copy(raw, 1048576)
            when (kind) {
                "count", "delete" -> integer(result, 0, 9007199254740991L, "query-result")
                "exists" -> { if (result !is Boolean) fail("query-result"); result }
                else -> {
                    if (result !is List<*> || result.size > (map(query["page"])["size"] as Long) || result.any { it !is Map<*, *> }) fail("query-result")
                    execution.check()
                    if (kind == "findOne") result.firstOrNull() else result
                }
            }
        }
        fun restClient(name: String): RestClient {
            if (name.isEmpty() || name.length > 128) fail("http-adapter")
            return RestClient(this, name)
        }
        internal fun restExchange(name: String, method: String, path: String, descriptor: Map<String, Any?>): Any? =
            operation { execution -> exchange(descriptor, Exchange(name, method, path), execution) }
        private fun exchange(descriptor: Any?, metadata: Exchange, execution: Execution): Any? {
            val request = map(copy(descriptor, 1048576)).toMutableMap()
            if (request.keys.any { it !in setOf("query", "headers", "body") }) fail("http-request")
            validateRequest(request)
            if (metadata.method in setOf("GET", "HEAD") && request.containsKey("body")) fail("http-request")
            request["method"] = metadata.method; request["path"] = metadata.path
            request["redirect"] = "error"; request["credentials"] = "omit"; request["maxResponseBytes"] = 1048576
            val adapter = configuration.http[metadata.client] ?: fail("http-adapter")
            return try { val value = adapter.exchange(java.util.Collections.unmodifiableMap(request), execution); execution.check(); copy(value, 1048576) }
            catch (error: Failure) { throw error } catch (_: RuntimeException) { fail("http-adapter") }
        }
        internal fun persist(entity: Entity, operation: String, key: String, value: Any?): Any? = operation { execution ->
            val adapter = entityAdapter(entity.adapter)
            val joined = OpenTransactions.of(entity.adapter)
            val command = if (joined == null) record("operation" to operation, "entity" to entity.name, "table" to entity.table, "idField" to entity.id, key to value)
                else record("operation" to operation, "entity" to entity.name, "table" to entity.table, "idField" to entity.id, key to value, "transaction" to joined)
            try { val result = adapter.execute(command, execution); execution.check(); copy(result, 1048576) }
            catch (error: Failure) { throw error } catch (_: RuntimeException) { fail("entity-adapter") }
        }
    }
    class RestClient internal constructor(private val context: Context, private val name: String) {
        fun get(path: String): RestCall = RestCall(context, name, "GET", path)
        fun post(path: String): RestCall = RestCall(context, name, "POST", path)
        fun put(path: String): RestCall = RestCall(context, name, "PUT", path)
        fun patch(path: String): RestCall = RestCall(context, name, "PATCH", path)
        fun delete(path: String): RestCall = RestCall(context, name, "DELETE", path)
        fun head(path: String): RestCall = RestCall(context, name, "HEAD", path)
        fun options(path: String): RestCall = RestCall(context, name, "OPTIONS", path)
    }
    class RestCall internal constructor(
        private val context: Context, private val name: String, private val method: String, private val path: String) {
        private val descriptor = mutableMapOf<String, Any?>("query" to mutableMapOf<String, Any?>(), "headers" to mutableMapOf<String, Any?>())
        init { if (!safePath(path)) fail("http-request") }
        @Suppress("UNCHECKED_CAST")
        fun query(values: Map<String, Any?>): RestCall {
            (descriptor["query"] as MutableMap<String, Any?>).putAll(values); return this
        }
        @Suppress("UNCHECKED_CAST")
        fun header(key: String, value: String): RestCall {
            (descriptor["headers"] as MutableMap<String, Any?>)[key] = value; return this
        }
        fun body(value: Any?): RestCall {
            if (method == "GET" || method == "HEAD") fail("http-request")
            descriptor["body"] = value; return this
        }
        fun retrieve(): Any? = context.restExchange(name, method, path, descriptor)
    }
    private fun safePath(path: String): Boolean {
        if (!path.startsWith("/") || path.startsWith("//") || path.length > 2048) return false
        if (path.any { it <= ' ' || it == '\\' || it == '?' || it == '#' || it == '%' }) return false
        return path.split("/").none { it == "." || it == ".." }
    }
    class Repository internal constructor(private val context: Context, private val entity: Entity) {
        fun find(input: Map<String, Any?> = emptyMap()): Any? = context.persist(entity, "find", "page", page(input, 20, 100, arrayOf(entity.id)))
        fun save(value: Any?): Any? { val data = map(copy(value, 1048576)); identifier(data[entity.id]); return context.persist(entity, "save", "value", data) }
        fun delete(id: Any?): Any? { identifier(id); return context.persist(entity, "delete", "id", id) }
    }
    private fun identifier(value: Any?) {
        if (value is String) { if (value.isEmpty() || bytes(value) > 256) fail("entity-id"); return }
        integer(value, -9007199254740991L, 9007199254740991L, "entity-id")
    }
    private fun queryDescriptor(kind: String, fields: Array<String>, where: Array<Array<Any?>>, orderBy: Array<Array<String>>, input: Any?, defaultSize: Int, maxSize: Int): Map<String, Any?> {
        if (kind !in setOf("find", "findOne", "count", "exists", "delete")) fail("query")
        val filters = where.map { predicate ->
            if (predicate.size !in 2..3 || predicate[0] !in fields || predicate[1] !is String) fail("query")
            val operator = predicate[1] as String
            if (operator == "isNull" || operator == "isNotNull") {
                if (predicate.size != 2) fail("query")
                record("field" to predicate[0], "operator" to operator)
            } else {
                if (predicate.size != 3) fail("query")
                val value = copy(predicate[2], 65536)
                if (operator == "in") {
                    if (value !is List<*> || value.size > 100) fail("query-binding")
                    value.forEach { queryScalar(it) }
                } else {
                    if (operator !in setOf("eq", "ne", "lt", "lte", "gt", "gte")) fail("query")
                    queryScalar(value)
                }
                record("field" to predicate[0], "operator" to operator, "value" to value)
            }
        }
        val bindings = copy(filters, 65536)
        val seen = mutableSetOf<String>()
        val ordering = orderBy.map { order ->
            if (order.size != 2 || order[0] !in fields || !seen.add(order[0]) || order[1] !in setOf("asc", "desc")) fail("query")
            record("field" to order[0], "direction" to order[1])
        }
        if (kind == "findOne") return map(copy(record("kind" to "find", "where" to bindings, "orderBy" to ordering, "page" to page(mapOf("size" to 1L), 1, 1, fields)), 1048576))
        if (kind != "find") return map(copy(record("kind" to kind, "where" to bindings, "orderBy" to ordering), 1048576))
        val page = page(input, defaultSize, maxSize, fields)
        if ((page["sort"] as List<*>).isNotEmpty()) fail("pagination")
        return map(copy(record("kind" to kind, "where" to bindings, "orderBy" to ordering, "page" to page), 1048576))
    }
    private fun queryScalar(value: Any?) {
        if (value != null && value !is String && value !is Number && value !is Boolean) fail("query-binding")
    }
    fun page(input: Any?, defaultSize: Int, maxSize: Int, allowed: Array<String>): Map<String, Any?> {
        val value = if (input == null) emptyMap() else map(copy(input, 65536))
        if (value.keys.any { it !in setOf("page", "size", "sort", "offset") }) fail("pagination")
        val page = integer(value["page"] ?: 0, 0, 1000000, "pagination")
        val size = integer(value["size"] ?: defaultSize, 1, maxSize.toLong(), "pagination")
        val offset = page * size
        if (offset > 1000000) fail("pagination")
        val sorts = value["sort"] ?: emptyList<Any?>()
        if (sorts !is List<*>) fail("pagination")
        val seen = mutableSetOf<String>()
        val normalized = sorts.map {
            if (it !is String) fail("pagination")
            val pair = it.split(':')
            if (pair.size > 2 || pair[0] !in allowed || !seen.add(pair[0])) fail("pagination")
            val direction = pair.getOrElse(1) { "asc" }
            if (direction != "asc" && direction != "desc") fail("pagination")
            pair[0] + ":" + direction
        }
        return record("page" to page, "size" to size, "offset" to offset, "sort" to java.util.Collections.unmodifiableList(normalized))
    }
    private fun validateRequest(request: Map<String, Any?>) {
        for (key in listOf("query", "headers")) {
            if (!request.containsKey(key)) continue
            val fields = map(request[key]); if (fields.size > 128) fail("http-request")
            for ((name, value) in fields) {
                if (name.isEmpty() || bytes(name) > 256) fail("http-request")
                if (key == "query") { if (value !is String && value !is Number && value !is Boolean) fail("http-request"); continue }
                if (value !is String || bytes(value) > 8192 || !name.matches(Regex("[!#\$%&'*+.^_`|~0-9A-Za-z-]{1,128}")) || '\r' in value || '\n' in value || '\u0000' in value) fail("http-request")
                if (name.lowercase(java.util.Locale.ROOT) in setOf("host", "connection", "content-length", "transfer-encoding", "cookie")) fail("http-request")
            }
        }
    }
    private fun integer(value: Any?, minimum: Long, maximum: Long, code: String): Long {
        if (value !is Byte && value !is Short && value !is Int && value !is Long) fail(code)
        val number = (value as Number).toLong()
        if (number !in minimum..maximum) fail(code)
        return number
    }
    private fun bytes(value: String): Int = value.toByteArray(Charsets.UTF_8).size
    private fun record(vararg pairs: Pair<String, Any?>): Map<String, Any?> = java.util.Collections.unmodifiableMap(linkedMapOf(*pairs))
    private fun map(value: Any?): Map<String, Any?> {
        if (value !is Map<*, *>) fail("json-value")
        val result = linkedMapOf<String, Any?>()
        for ((key, item) in value) { if (key !is String) fail("json-value"); result[key] = item }
        return java.util.Collections.unmodifiableMap(result)
    }
    private val gatewayFailures = setOf("invalid", "timeout", "capacity", "adapter")
    fun gatewayResult(answer: Map<*, *>): Any? {
        if (answer.containsKey("result")) return answer["result"]
        val kind = answer["error"]
        if (kind is String && kind in gatewayFailures) throw gatewayFailure(kind)
        throw gatewayFailure("adapter")
    }
    private fun gatewayFailure(kind: String): Failure = Failure(kind, null, "Gateway call failed.")
    fun mapValue(value: Any?, plan: Array<Array<Any?>>): Any? {
        val source = map(copy(value, 1048576))
        val output = linkedMapOf<String, Any?>()
        for (entry in plan) {
            val path = entry[1]
            if (path == null) { output[entry[0] as String] = entry[2]; continue }
            var current: Any? = source
            @Suppress("UNCHECKED_CAST")
            for (name in path as Array<String>) { current = (current as? Map<*, *>)?.get(name) }
            output[entry[0] as String] = current
        }
        return copy(output, 1048576)
    }
    fun json(value: Any?, limit: Int = 1048576): String = encode(copy(value, limit))
    fun copy(value: Any?, limit: Int): Any? {
        if (limit !in 1..1048576) fail("json-size")
        val result = copyValue(value, java.util.IdentityHashMap(), 0, intArrayOf(0, 0), limit)
        if (bytes(encode(result)) > limit) fail("json-size")
        return result
    }
    private fun copyValue(value: Any?, seen: java.util.IdentityHashMap<Any, Boolean>, depth: Int, budget: IntArray, limit: Int): Any? {
        budget[0]++; budget[1] += 1
        if (depth > 32 || budget[0] > 65536 || budget[1] > limit) fail("json-size")
        when (value) {
            null, is Boolean -> return value
            is String -> { budget[1] += bytes(value); if (budget[1] > limit) fail("json-size"); return value }
            is Byte, is Short, is Int, is Long -> { integer(value, -9007199254740991L, 9007199254740991L, "json-number"); return value }
            is Double, is Float -> { if (!(value as Number).toDouble().isFinite()) fail("json-number"); return value }
        }
        if (value == null) return null
        if (seen.put(value, true) != null) fail("json-cycle")
        try {
            if (value is List<*>) {
                if (value.size > 65536) fail("json-size")
                return java.util.Collections.unmodifiableList(value.map { copyValue(it, seen, depth + 1, budget, limit) })
            }
            if (value is Map<*, *>) {
                if (value.size > 65536) fail("json-size")
                val result = sortedMapOf<String, Any?>()
                for ((key, item) in value) {
                    if (key !is String) fail("json-value")
                    budget[1] += bytes(key); if (budget[1] > limit) fail("json-size")
                    result[key] = copyValue(item, seen, depth + 1, budget, limit)
                }
                return java.util.Collections.unmodifiableMap(result)
            }
            fail("json-value")
        } finally { seen.remove(value) }
    }
    private fun encode(value: Any?): String = when (value) {
        null -> "null"
        is String -> quote(value)
        is Number, is Boolean -> value.toString()
        is List<*> -> value.joinToString(",", "[", "]") { encode(it) }
        else -> map(value).entries.joinToString(",", "{", "}") { quote(it.key) + ":" + encode(it.value) }
    }
    private fun quote(value: String): String {
        val output = StringBuilder("\"")
        var index = 0
        while (index < value.length) {
            val ch = value[index++]
            when {
                Character.isHighSurrogate(ch) -> { if (index >= value.length || !Character.isLowSurrogate(value[index])) fail("json-string"); output.append(ch).append(value[index++]) }
                Character.isLowSurrogate(ch) -> fail("json-string")
                ch == '"' || ch == '\\' -> output.append('\\').append(ch)
                ch.code < 32 -> output.append("\\u").append(ch.code.toString(16).padStart(4, '0'))
                else -> output.append(ch)
            }
        }
        return output.append('"').toString()
    }
}

object BehaviorCalls {
    private val keys = linkedSetOf<String>()

    @Synchronized
    fun record(key: String) { keys.add(key) }

    @Synchronized
    fun take(): List<String> {
        val taken = keys.toList()
        keys.clear()
        return taken
    }
}

infix fun <T> Unit.behaviorThen(value: T): T = value
