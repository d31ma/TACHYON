class DataSource {
    fun query(sql: String, parameters: List<Any?>): Any? = YonManagedResources.call(mapOf("operation" to "sql", "sql" to sql, "parameters" to parameters, "mode" to "rows"))
    fun execute(sql: String, parameters: List<Any?>): Any? = YonManagedResources.call(mapOf("operation" to "sql", "sql" to sql, "parameters" to parameters, "mode" to "execute"))
    fun execute(command: Map<String, Any?>): Any? = this.command(command)
    fun command(command: Map<String, Any?>): Any? = YonManagedResources.call(command)
    fun findById(collection: String, id: String): Any? = document("findById", mapOf("collection" to collection, "id" to id))
    fun insert(collection: String, document: Map<String, Any?>): Any? = document("insert", mapOf("collection" to collection, "value" to document))
    fun patch(collection: String, id: String, fields: Map<String, Any?>): Any? = document("patch", mapOf("collection" to collection, "id" to id, "value" to fields))
    fun deleteById(collection: String, id: String): Any? = document("deleteById", mapOf("collection" to collection, "id" to id))
    fun scroll(collection: String, where: Map<String, Any?>? = null, cursor: String? = null, size: Int = 20): Any? =
        YonManagedResources.call(mapOf("operation" to "document", "command" to "scroll", "collection" to collection, "where" to where, "cursor" to cursor, "size" to size), YonManagedResources.SCROLL_TIMEOUT_NANOSECONDS)
    fun createCollection(name: String): Any? = document("createCollection", mapOf("collection" to name))
    fun dropCollection(name: String): Any? = document("dropCollection", mapOf("collection" to name))
    fun collectionExists(name: String): Any? = document("collectionExists", mapOf("collection" to name))
    fun publish(topic: String, payload: Any?, key: String? = null): Any? = queue("publish", mapOf("topic" to topic, "payload" to payload, "key" to key))
    fun claim(topic: String, group: String, maxMessages: Int = 1, leaseMs: Int = 30_000): Any? =
        queue("claim", mapOf("topic" to topic, "group" to group, "maxMessages" to maxMessages, "leaseMs" to leaseMs))
    fun ack(topic: String, group: String, delivery: Map<*, *>): Any? =
        queue("ack", mapOf("topic" to topic, "group" to group, "id" to delivery["id"], "receipt" to delivery["receipt"]))
    fun nack(topic: String, group: String, delivery: Map<*, *>, delayMs: Int = 0): Any? =
        queue("nack", mapOf("topic" to topic, "group" to group, "id" to delivery["id"], "receipt" to delivery["receipt"], "delayMs" to delayMs))
    private fun document(command: String, fields: Map<String, Any?>): Any? = YonManagedResources.call(mapOf("operation" to "document", "command" to command) + fields)
    private fun queue(command: String, fields: Map<String, Any?>): Any? = YonManagedResources.call(mapOf("operation" to "queue", "command" to command) + fields)
}

class RedisTemplate {
    fun get(key: String): Any? = YonManagedResources.call(mapOf("operation" to "redis", "command" to "get", "key" to key))
    fun delete(key: String): Any? = YonManagedResources.call(mapOf("operation" to "redis", "command" to "delete", "key" to key))
    fun set(key: String, value: Any?, ttlSeconds: Long): Any? = YonManagedResources.call(mapOf("operation" to "redis", "command" to "set", "key" to key, "value" to value, "ttlSeconds" to ttlSeconds))
}

private object YonManagedResources {
    const val LIMIT = 1024 * 1024
    private const val RESOURCE_TIMEOUT_NANOSECONDS = 5_000_000_000L
    const val SCROLL_TIMEOUT_NANOSECONDS = 30_000_000_000L
    private const val CONNECT_TIMEOUT_MILLISECONDS = 5000L
    private fun failed(): Nothing = throw IllegalStateException("Yon managed resource operation failed.")
    fun call(command: Any?, timeoutNanoseconds: Long = RESOURCE_TIMEOUT_NANOSECONDS): Any? {
        val text = try { Json.stringify(command) } catch (_: Exception) { failed() }
        return exchange(text, timeoutNanoseconds)
    }
    fun exchange(text: String, timeoutNanoseconds: Long): Any? {
        try {
            val endpoint = System.getenv("YON_RESOURCE_ENDPOINT") ?: failed()
            val token = System.getenv("YON_RESOURCE_TOKEN") ?: failed()
            if (!Regex("http://127\\.0\\.0\\.1:[0-9]{1,5}/").matches(endpoint) || !Regex("[0-9a-f]{64}").matches(token)) failed()
            val port = endpoint.substring(17, endpoint.length - 1).toInt()
            if (port !in 1..65535) failed()
            val body = text.toByteArray(Charsets.UTF_8)
            if (body.size > LIMIT) failed()
            val deadline = System.nanoTime() + timeoutNanoseconds
            val connectTimeout = minOf(CONNECT_TIMEOUT_MILLISECONDS, timeoutNanoseconds / 1_000_000L).toInt()
            return java.net.Socket().use { socket ->
                socket.connect(java.net.InetSocketAddress("127.0.0.1", port), connectTimeout)
                val headers = "POST / HTTP/1.1\r\nHost: 127.0.0.1:$port\r\nAuthorization: Bearer $token\r\nContent-Type: application/json\r\nContent-Length: ${body.size}\r\nConnection: close\r\n\r\n"
                socket.getOutputStream().apply {
                    write(headers.toByteArray(Charsets.US_ASCII)); write(body); flush()
                }
                val input = socket.getInputStream()
                val head = java.io.ByteArrayOutputStream()
                val delimiter = intArrayOf(13, 10, 13, 10)
                var matched = 0
                while (matched < 4) {
                    timeout(socket, deadline)
                    val next = input.read()
                    if (next < 0 || head.size() >= 16384) failed()
                    head.write(next)
                    matched = if (next == delimiter[matched]) matched + 1 else if (next == 13) 1 else 0
                }
                val lines = head.toString(Charsets.US_ASCII.name()).split("\r\n")
                if (!Regex("HTTP/1\\.[01] 200(?: .*)?").matches(lines.first())) failed()
                var length = -1
                for (line in lines.drop(1).map { it.lowercase(java.util.Locale.ROOT) }) {
                    if (line.startsWith("transfer-encoding:")) failed()
                    if (line.startsWith("content-length:")) {
                        val value = line.substring(15).trim()
                        if (length >= 0 || !Regex("[0-9]{1,7}").matches(value)) failed()
                        length = value.toInt()
                    }
                }
                if (length !in 0..LIMIT) failed()
                val response = ByteArray(length)
                var read = 0
                while (read < length) {
                    timeout(socket, deadline)
                    val count = input.read(response, read, length - read)
                    if (count <= 0) failed()
                    read += count
                }
                Json.parse(Charsets.UTF_8.newDecoder().decode(java.nio.ByteBuffer.wrap(response)).toString())
            }
        } catch (_: Exception) { failed() }
    }
    private fun timeout(socket: java.net.Socket, deadline: Long) {
        val remaining = (deadline - System.nanoTime()) / 1_000_000L
        if (remaining <= 0) failed()
        socket.soTimeout = minOf(Int.MAX_VALUE.toLong(), remaining).toInt()
    }
}

object YonGateway {
    private const val ANSWER_MARGIN_NANOSECONDS = 1_000_000_000L
    fun call(key: String, arguments: Array<Any?>): Map<*, *> {
        val body = try {
            Json.stringify(mapOf("operation" to "gateway", "key" to key, "arguments" to arguments))
        } catch (_: Exception) {
            return failure("invalid")
        }
        if (body.toByteArray(Charsets.UTF_8).size > YonManagedResources.LIMIT) return failure("invalid")
        val timeLeft = Yon.timeLeftNanos()
        if (timeLeft <= 0) return failure("timeout")
        val answer = try {
            YonManagedResources.exchange(body, timeLeft + ANSWER_MARGIN_NANOSECONDS)
        } catch (_: Exception) {
            if (Yon.timeLeftNanos() <= 0) return failure("timeout")
            return failure("adapter")
        }
        if (answer !is Map<*, *>) return failure("adapter")
        if (answer.containsKey("result") || answer["error"] is String) return answer
        return failure("adapter")
    }
    private fun failure(kind: String): Map<String, Any?> = mapOf("error" to kind)
}

object Json {
    private const val MAX_CHARS = 16 * 1024 * 1024
    private const val MAX_DEPTH = 64
    private val NUMBER = Regex("-?(?:0|[1-9][0-9]*)(?:\\.[0-9]+)?(?:[eE][+-]?[0-9]+)?")

    fun parse(source: String?): Any? {
        require(source != null && source.length <= MAX_CHARS) { "Invalid JSON document." }
        return Reader(source).document()
    }
    fun stringify(value: Any?): String = StringBuilder().also { append(it, value, 0) }.toString()
    fun write(value: Any?): String = stringify(value)
    fun quote(text: String): String = StringBuilder().also { quoted(it, text) }.toString()

    private fun check(out: StringBuilder) {
        require(out.length <= MAX_CHARS) { "JSON document exceeds its limit." }
    }
    private fun append(out: StringBuilder, value: Any?, depth: Int) {
        require(depth <= MAX_DEPTH) { "JSON nesting exceeds its limit." }
        when (value) {
            null -> out.append("null")
            is String -> quoted(out, value)
            is Boolean -> out.append(value)
            is Number -> {
                val text = value.toString()
                require(NUMBER.matches(text)) { "Unsupported JSON number." }
                out.append(text)
            }
            is Map<*, *> -> {
                out.append('{')
                var first = true
                for ((key, item) in value) {
                    require(key is String) { "JSON object keys must be strings." }
                    if (!first) out.append(',')
                    first = false
                    quoted(out, key)
                    out.append(':')
                    append(out, item, depth + 1)
                }
                out.append('}')
            }
            is Iterable<*> -> {
                out.append('[')
                var first = true
                for (item in value) {
                    if (!first) out.append(',')
                    first = false
                    append(out, item, depth + 1)
                }
                out.append(']')
            }
            is Enum<*> -> quoted(out, value.name)
            is __YonAnswer -> throw IllegalArgumentException("Unsupported JSON value.")
            else -> {
                if (!value.javaClass.isArray) {
                    append(out, YonModels.plain(value), depth + 1)
                    return
                }
                out.append('[')
                for (index in 0 until java.lang.reflect.Array.getLength(value)) {
                    if (index != 0) out.append(',')
                    append(out, java.lang.reflect.Array.get(value, index), depth + 1)
                }
                out.append(']')
            }
        }
        check(out)
    }
    private fun quoted(out: StringBuilder, text: String) {
        out.append('"')
        for (c in text) {
            when (c) {
                '"' -> out.append("\\\"")
                '\\' -> out.append("\\\\")
                '\n' -> out.append("\\n")
                '\r' -> out.append("\\r")
                '\t' -> out.append("\\t")
                else -> if (c.code < 0x20 || c.isSurrogate()) out.append("\\u%04x".format(c.code))
                        else out.append(c)
            }
            check(out)
        }
        out.append('"')
    }
    private class Reader(private val source: String) {
        private var at = 0
        private fun invalid(): Nothing = throw IllegalArgumentException("Invalid JSON document.")
        private fun skip() { while (at < source.length && source[at] in " \n\r\t") at++ }
        private fun take(c: Char): Boolean {
            skip()
            if (at >= source.length || source[at] != c) return false
            at++
            return true
        }
        private fun expect(c: Char) { if (!take(c)) invalid() }
        fun document(): Any? {
            val result = value(0)
            skip()
            if (at != source.length) invalid()
            return result
        }
        private fun literal(text: String, result: Any?): Any? {
            if (!source.startsWith(text, at)) invalid()
            at += text.length
            return result
        }
        private fun value(depth: Int): Any? {
            skip()
            if (depth > MAX_DEPTH || at >= source.length) invalid()
            return when (source[at]) {
                '{' -> obj(depth)
                '[' -> list(depth)
                '"' -> string()
                't' -> literal("true", true)
                'f' -> literal("false", false)
                'n' -> literal("null", null)
                else -> number()
            }
        }
        private fun obj(depth: Int): Map<String, Any?> {
            expect('{')
            val entries = LinkedHashMap<String, Any?>()
            if (take('}')) return entries
            while (true) {
                skip()
                val key = string()
                expect(':')
                entries[key] = value(depth + 1)
                if (take('}')) return entries
                expect(',')
            }
        }
        private fun list(depth: Int): List<Any?> {
            expect('[')
            val values = ArrayList<Any?>()
            if (take(']')) return values
            while (true) {
                values.add(value(depth + 1))
                if (take(']')) return values
                expect(',')
            }
        }
        private fun string(): String {
            expect('"')
            val out = StringBuilder()
            while (at < source.length) {
                val c = source[at++]
                if (c == '"') return out.toString()
                if (c.code < 0x20) invalid()
                if (c != '\\') { out.append(c); continue }
                if (at >= source.length) invalid()
                when (val escaped = source[at++]) {
                    '"', '\\', '/' -> out.append(escaped)
                    'n' -> out.append('\n')
                    'r' -> out.append('\r')
                    't' -> out.append('\t')
                    'b' -> out.append('\b')
                    'f' -> out.append('\u000c')
                    'u' -> {
                        if (at + 4 > source.length) invalid()
                        val digits = source.substring(at, at + 4)
                        if (!digits.matches(Regex("[0-9a-fA-F]{4}"))) invalid()
                        out.append(digits.toInt(16).toChar())
                        at += 4
                    }
                    else -> invalid()
                }
            }
            invalid()
        }
        private fun number(): Number {
            val match = NUMBER.find(source, at) ?: invalid()
            if (match.range.first != at) invalid()
            at = match.range.last + 1
            val digits = match.value
            return digits.toLongOrNull() ?: digits.toBigDecimal()
        }
    }
}

class __YonFrame(parsed: Any?) {
    @Suppress("UNCHECKED_CAST")
    private val raw: Map<String, Any?> = parsed as? Map<String, Any?> ?: emptyMap()

    fun method(): String = raw["method"]?.toString() ?: ""

    fun route(): String = raw["route"]?.toString() ?: ""

    @Suppress("UNCHECKED_CAST")
    fun parameter(name: String): String =
        (raw["parameters"] as? Map<String, Any?>)?.get(name)?.toString() ?: ""

    @Suppress("UNCHECKED_CAST")
    fun body(): String = (raw["body"] as? Map<String, Any?>)?.get("data")?.toString() ?: ""

    fun raw(): Map<String, Any?> = raw

    @Suppress("UNCHECKED_CAST")
    fun arguments(): List<Any?> = raw["arguments"] as? List<Any?> ?: emptyList()

    @Suppress("UNCHECKED_CAST")
    fun headers(): Map<String, List<String>> = raw["headers"] as? Map<String, List<String>> ?: emptyMap()

    fun target(): String = raw["target"]?.toString() ?: route()
}

class YonExchange(frame: __YonFrame) : com.sun.net.httpserver.HttpExchange() {
    private val requestHeaders = com.sun.net.httpserver.Headers()
    private val responseHeaders = com.sun.net.httpserver.Headers()
    private val written = java.io.ByteArrayOutputStream()
    private val attributes = HashMap<String, Any?>()
    private val uri = java.net.URI.create(frame.target())
    private val method = frame.method()
    private val body = frame.body().toByteArray()
    private var responseCode = -1

    init {
        frame.headers().forEach { (name, values) -> values.forEach { requestHeaders.add(name, it) } }
    }

    override fun getRequestHeaders(): com.sun.net.httpserver.Headers = requestHeaders
    override fun getResponseHeaders(): com.sun.net.httpserver.Headers = responseHeaders
    override fun getRequestURI(): java.net.URI = uri
    override fun getRequestMethod(): String = method
    override fun getHttpContext(): com.sun.net.httpserver.HttpContext? = null
    override fun close() {}
    override fun getRequestBody(): java.io.InputStream = java.io.ByteArrayInputStream(body)
    override fun getResponseBody(): java.io.OutputStream = written
    override fun sendResponseHeaders(code: Int, length: Long) {
        responseCode = code
    }
    override fun getRemoteAddress(): java.net.InetSocketAddress = java.net.InetSocketAddress(clientAddress(), 0)
    private fun clientAddress(): java.net.InetAddress =
        requestHeaders.getFirst("X-Real-IP")?.let { runCatching { java.net.InetAddress.getByName(it) }.getOrNull() }
            ?: java.net.InetAddress.getLoopbackAddress()
    override fun getResponseCode(): Int = responseCode
    override fun getLocalAddress(): java.net.InetSocketAddress = java.net.InetSocketAddress(java.net.InetAddress.getLoopbackAddress(), 0)
    override fun getProtocol(): String = "HTTP/1.1"
    override fun getAttribute(name: String): Any? = attributes[name]
    override fun setAttribute(name: String, value: Any?) {
        attributes[name] = value
    }
    override fun setStreams(input: java.io.InputStream?, output: java.io.OutputStream?) {}
    override fun getPrincipal(): com.sun.net.httpserver.HttpPrincipal? = null

    internal fun answered(): Boolean = responseCode >= 0

    internal fun answer(): __YonAnswer {
        val headers = LinkedHashMap<String, List<String>>()
        responseHeaders.forEach { (name, values) -> headers[name.lowercase()] = values }
        return __YonAnswer.json(written.toString(Charsets.UTF_8)).headers(headers).status(responseCode)
    }
}

object YonModels {
    fun argument(owner: Class<*>, method: String, count: Int, index: Int, value: Any?): Any? {
        val found = owner.declaredMethods.firstOrNull { it.name == method && it.parameterCount == count }
            ?: return value
        return read(found.genericParameterTypes[index], value)
    }

    fun read(type: java.lang.reflect.Type, value: Any?): Any? {
        if (type is java.lang.reflect.WildcardType) return read(type.upperBounds[0], value)
        if (type is java.lang.reflect.ParameterizedType) return readGeneric(type, value)
        val kind = type as? Class<*> ?: return value
        if (value == null) return zero(kind)
        if (value is Number) return number(kind, value)
        if (kind.isEnum) return kind.enumConstants.firstOrNull { (it as Enum<*>).name == value.toString() }
        val map = value as? Map<*, *> ?: return value
        if (kind == Any::class.java || Map::class.java.isAssignableFrom(kind)) return value
        return construct(kind, map)
    }

    private fun readGeneric(type: java.lang.reflect.ParameterizedType, value: Any?): Any? {
        val raw = type.rawType as Class<*>
        val list = value as? List<*> ?: return value
        if (!Iterable::class.java.isAssignableFrom(raw)) return value
        val items = list.map { read(type.actualTypeArguments[0], it) }
        if (Set::class.java.isAssignableFrom(raw)) return items.toSet()
        return items
    }

    private fun zero(kind: Class<*>): Any? = when (kind) {
        java.lang.Boolean.TYPE -> false
        java.lang.Long.TYPE, java.lang.Integer.TYPE, java.lang.Short.TYPE, java.lang.Byte.TYPE,
        java.lang.Double.TYPE, java.lang.Float.TYPE -> number(kind, 0L)
        else -> null
    }

    private fun number(kind: Class<*>, number: Number): Any = when (kind) {
        java.lang.Integer.TYPE, java.lang.Integer::class.java -> number.toInt()
        java.lang.Long.TYPE, java.lang.Long::class.java -> number.toLong()
        java.lang.Short.TYPE, java.lang.Short::class.java -> number.toShort()
        java.lang.Byte.TYPE, java.lang.Byte::class.java -> number.toByte()
        java.lang.Double.TYPE, java.lang.Double::class.java -> number.toDouble()
        java.lang.Float.TYPE, java.lang.Float::class.java -> number.toFloat()
        else -> number
    }

    private fun fields(kind: Class<*>): List<java.lang.reflect.Field> =
        kind.declaredFields.filter { !java.lang.reflect.Modifier.isStatic(it.modifiers) && !it.isSynthetic }

    private fun construct(kind: Class<*>, map: Map<*, *>): Any? {
        val fields = fields(kind)
        val constructor = kind.declaredConstructors.firstOrNull { !it.isSynthetic && it.parameterCount == fields.size }
            ?: return value(kind, fields, map)
        constructor.isAccessible = true
        val arguments = Array(fields.size) { index -> read(constructor.genericParameterTypes[index], map[fields[index].name]) }
        return constructor.newInstance(*arguments)
    }

    private fun value(kind: Class<*>, fields: List<java.lang.reflect.Field>, map: Map<*, *>): Any? {
        val constructor = kind.getDeclaredConstructor()
        constructor.isAccessible = true
        val model = constructor.newInstance()
        for (field in fields) {
            if (!map.containsKey(field.name)) continue
            field.isAccessible = true
            field.set(model, read(field.genericType, map[field.name]))
        }
        return model
    }

    fun plain(model: Any): Map<String, Any?> {
        val result = LinkedHashMap<String, Any?>()
        for (field in fields(model.javaClass)) {
            field.isAccessible = true
            result[field.name] = field.get(model)
        }
        return result
    }
}

class __YonAnswer private constructor(private var status: Int, private val body: String) {
    private val headers = LinkedHashMap<String, MutableList<String>>()

    companion object {
        fun json(body: String): __YonAnswer {
            val response = __YonAnswer(200, body)
            response.headers["content-type"] = mutableListOf("application/json")
            return response
        }

        fun empty(status: Int): __YonAnswer = __YonAnswer(status, "")
    }

    fun status(value: Int): __YonAnswer {
        status = value
        return this
    }

    fun headers(replacement: Map<String, List<String>>): __YonAnswer {
        headers.clear()
        replacement.forEach { (name, values) -> headers[name] = values.toMutableList() }
        return this
    }

    fun header(name: String, value: String): __YonAnswer {
        headers.getOrPut(name) { mutableListOf() }.add(value)
        return this
    }

    fun writeTo(exchange: com.sun.net.httpserver.HttpExchange) {
        headers.forEach { (name, values) -> values.forEach { exchange.responseHeaders.add(name, it) } }
        val payload = body.toByteArray()
        exchange.sendResponseHeaders(status, if (payload.isEmpty()) -1 else payload.size.toLong())
        exchange.responseBody.write(payload)
    }

    fun envelope(): String =
        Json.write(mapOf("status" to status, "headers" to headers, "body" to body))
}

object Yon {
    private const val MAX_RELAY_STDOUT_BYTES = 16 * 1024 * 1024
    private const val MAX_RELAY_STDERR_BYTES = 64 * 1024
    private const val DEFAULT_DEADLINE_MILLISECONDS = 30_000L
    private const val NANOSECONDS_PER_MILLISECOND = 1_000_000L
    private const val FAILURE_MESSAGE = "Component invocation failed."
    private var startNanos = 0L
    private var relayEnvironment: Map<String, String>? = null
    private var deadlineNanos: Long? = null
    private var requestId = "unknown"
    private val daemonThreads = java.util.concurrent.Executor { task ->
        kotlin.concurrent.thread(isDaemon = true) { task.run() }
    }
    private data class RelayOutput(val bytes: ByteArray, val overflow: Boolean)

    private fun drain(pipe: java.io.InputStream, limit: Int): RelayOutput {
        val kept = java.io.ByteArrayOutputStream(minOf(limit, 8192))
        val chunk = ByteArray(8192)
        var overflow = false
        pipe.use {
            while (true) {
                val count = try { it.read(chunk) } catch (_: java.io.IOException) {
                    overflow = true
                    break
                }
                if (count < 0) break
                val remaining = maxOf(0, limit - kept.size())
                kept.write(chunk, 0, minOf(count, remaining))
                overflow = overflow || count > remaining
            }
        }
        return RelayOutput(kept.toByteArray(), overflow)
    }

    private fun killTree(child: Process) {
        child.descendants().forEach { it.destroyForcibly() }
        child.destroyForcibly()
    }

    internal fun captureStart() {
        if (relayEnvironment != null) return
        val uptimeMilliseconds = java.lang.management.ManagementFactory.getRuntimeMXBean().uptime
        startNanos = System.nanoTime() - uptimeMilliseconds * NANOSECONDS_PER_MILLISECOND
        relayEnvironment = relayEnvironmentFrom(System.getenv())
    }

    private fun relayEnvironmentFrom(environment: Map<String, String>): Map<String, String> {
        val kept = HashMap<String, String>()
        for ((name, value) in environment) {
            val upper = name.uppercase(java.util.Locale.ROOT)
            if (upper == "YON_APPLICATION_JSON" || upper.startsWith("YON_RESOURCE_")) continue
            kept[name] = value
        }
        return kept
    }

    internal fun captureInvocation(request: __YonFrame) {
        if (deadlineNanos != null) return
        val id = request.raw()["request_id"]
        if (id is String) requestId = id
        deadlineNanos = startNanos + invocationDeadlineMilliseconds(request) * NANOSECONDS_PER_MILLISECOND
    }

    private fun invocationDeadlineMilliseconds(request: __YonFrame): Long {
        val requested = request.raw()["deadline_ms"]
        if (requested is Number) return requested.toLong()
        return DEFAULT_DEADLINE_MILLISECONDS
    }

    internal fun renewDeadline() {
        deadlineNanos = System.nanoTime() + DEFAULT_DEADLINE_MILLISECONDS * NANOSECONDS_PER_MILLISECOND
    }

    internal fun timeLeftNanos(): Long {
        val deadline = deadlineNanos ?: return 0
        return deadline - System.nanoTime()
    }

    private var exchange: YonExchange? = null

    @JvmStatic
    fun exchange(frame: __YonFrame): YonExchange = exchange ?: YonExchange(frame).also { exchange = it }

    internal fun answered(): __YonAnswer? = exchange?.takeIf { it.answered() }?.answer()

    @JvmStatic
    fun relay(command: List<String>, exchange: com.sun.net.httpserver.HttpExchange) {
        relayAnswer(command, exchange).writeTo(exchange)
    }

    private fun relayMessage(exchange: com.sun.net.httpserver.HttpExchange): Map<String, Any?> {
        val headers = LinkedHashMap<String, Any?>()
        exchange.requestHeaders.forEach { (name, values) -> headers[name.lowercase()] = values }
        val body = String(exchange.requestBody.readAllBytes(), Charsets.UTF_8)
        return linkedMapOf(
            "route" to exchange.requestURI.toString(),
            "method" to exchange.requestMethod,
            "headers" to headers,
            "body" to mapOf("encoding" to "utf8", "data" to body),
        )
    }

    private fun relayAnswer(command: List<String>, exchange: com.sun.net.httpserver.HttpExchange): __YonAnswer {
        if (command.isEmpty()) return relayFailed("start")
        if (timeLeftNanos() <= 0) return relayFailed("timeout")
        val payload = try {
            Json.write(relayMessage(exchange)).toByteArray()
        } catch (_: IllegalArgumentException) {
            return relayFailed("protocol")
        }
        val child = try {
            startChild(command)
        } catch (_: Exception) {
            return relayFailed("start")
        }
        val stdoutTask = java.util.concurrent.CompletableFuture.supplyAsync(
            { drain(child.inputStream, MAX_RELAY_STDOUT_BYTES) },
            daemonThreads,
        )
        val stderrTask = java.util.concurrent.CompletableFuture.supplyAsync(
            { drain(child.errorStream, MAX_RELAY_STDERR_BYTES) },
            daemonThreads,
        )
        daemonThreads.execute { sendRequest(child, payload) }
        return awaitChild(child, stdoutTask, stderrTask)
    }

    private fun startChild(command: List<String>): Process {
        val builder = ProcessBuilder(command)
        builder.environment().clear()
        builder.environment().putAll(relayEnvironment ?: emptyMap())
        return builder.start()
    }

    private fun sendRequest(child: Process, payload: ByteArray) {
        try {
            child.outputStream.use { it.write(payload) }
        } catch (_: java.io.IOException) {
        }
    }

    private fun awaitChild(
        child: Process,
        stdoutTask: java.util.concurrent.CompletableFuture<RelayOutput>,
        stderrTask: java.util.concurrent.CompletableFuture<RelayOutput>,
    ): __YonAnswer {
        val nanoseconds = java.util.concurrent.TimeUnit.NANOSECONDS
        try {
            if (!child.waitFor(timeLeftNanos(), nanoseconds)) return stopChild(child)
            val stdout = stdoutTask.get(timeLeftNanos(), nanoseconds)
            val stderr = stderrTask.get(timeLeftNanos(), nanoseconds)
            return relayResult(child.exitValue(), stdout, stderr)
        } catch (_: java.util.concurrent.TimeoutException) {
            return stopChild(child)
        } catch (_: InterruptedException) {
            Thread.currentThread().interrupt()
            return stopChild(child)
        } catch (_: java.util.concurrent.ExecutionException) {
            return relayFailed("overflow")
        }
    }

    private fun stopChild(child: Process): __YonAnswer {
        killTree(child)
        closePipe(child.inputStream)
        closePipe(child.errorStream)
        return relayFailed("timeout")
    }

    private fun closePipe(pipe: java.io.InputStream) {
        try {
            pipe.close()
        } catch (_: java.io.IOException) {
        }
    }

    private fun relayResult(exitCode: Int, stdout: RelayOutput, stderr: RelayOutput): __YonAnswer {
        if (stdout.overflow || stderr.overflow) return relayFailed("overflow")
        if (exitCode != 0) return relayFailed("exit")
        val parsed = try {
            Json.parse(stdout.bytes.decodeToString())
        } catch (_: IllegalArgumentException) {
            return relayFailed("protocol")
        }
        val envelope = parsed as? Map<*, *> ?: return relayFailed("protocol")
        return responseFromEnvelope(envelope)
    }

    private fun responseFromEnvelope(parsed: Map<*, *>): __YonAnswer {
        var response = __YonAnswer.json(parsed["body"]?.toString() ?: "")
        (parsed["status"] as? Number)?.let { response = response.status(it.toInt()) }
        val written = parsed["headers"] as? Map<*, *>
        if (!written.isNullOrEmpty()) {
            response = response.headers(emptyMap())
            written.forEach { (name, values) ->
                val list = values as? List<*> ?: listOf(values)
                list.forEach { response = response.header(name.toString(), it.toString()) }
            }
        }
        return response
    }

    private fun relayFailed(category: String): __YonAnswer {
        val event = linkedMapOf("event" to "handler.relay_failed", "request_id" to requestId, "category" to category)
        System.err.print(Json.write(event) + "\n")
        System.err.flush()
        return __YonAnswer.json(Json.write(mapOf("error" to FAILURE_MESSAGE))).status(502)
    }
}

object YonStream {
    private const val MAX_FRAME_BYTES = 16 * 1024 * 1024

    fun event(requestId: String, value: Any?): Boolean = frame(
        requestId,
        mapOf(
            "protocol_version" to 1,
            "kind" to "event",
            "request_id" to requestId,
            "body" to mapOf("encoding" to "utf8", "data" to Json.write(value)),
        ),
    )

    fun failure(requestId: String) {
        frame(
            requestId,
            mapOf(
                "protocol_version" to 1,
                "kind" to "response",
                "request_id" to requestId,
                "status" to 500,
                "headers" to emptyMap<String, List<String>>(),
                "error" to mapOf(
                    "code" to "TY2204",
                    "message" to "The handler stream failed.",
                    "retryable" to false,
                ),
            ),
        )
    }

    private fun frame(requestId: String, frame: Map<String, Any?>): Boolean {
        val payload = Json.write(frame).toByteArray(Charsets.UTF_8)
        if (payload.size > MAX_FRAME_BYTES) {
            val refusal = Json.write(
                mapOf(
                    "protocol_version" to 1,
                    "kind" to "response",
                    "request_id" to requestId,
                    "status" to 500,
                    "headers" to emptyMap<String, List<String>>(),
                    "error" to mapOf(
                        "code" to "TY2203",
                        "message" to "Streamed event exceeds the protocol frame limit.",
                        "retryable" to false,
                    ),
                ),
            ).toByteArray(Charsets.UTF_8)
            write(refusal)
            return false
        }
        write(payload)
        return true
    }

    private fun write(payload: ByteArray) {
        val out = System.out
        out.write(
            byteArrayOf(
                (payload.size ushr 24).toByte(),
                (payload.size ushr 16).toByte(),
                (payload.size ushr 8).toByte(),
                payload.size.toByte(),
            ),
        )
        out.write(payload)
        out.flush()
    }
}

interface YonSocketEndpoint {
    fun open(session: SocketSession)
    fun message(session: SocketSession, value: Any?)
    fun close(session: SocketSession)
    fun error(session: SocketSession, error: Exception)
}

class SocketPeer internal constructor(private val session: SocketSession, val id: String) {
    fun send(message: Any?) = session.send(message, id)
}

class SocketSession internal constructor(val id: String) {
    private val inbox = ArrayDeque<Any?>()
    private var closed = false
    private var sessions: List<*>? = null

    fun send(message: Any?) = send(message, null)

    fun close() {
        closed = true
        write(mapOf("kind" to "close"))
    }

    fun openSessions(): List<SocketPeer> {
        sessions = null
        write(mapOf("kind" to "sessions"))
        while (sessions == null && !closed) receive()
        return sessions.orEmpty().map { SocketPeer(this, it.toString()) }
    }

    internal fun send(message: Any?, to: String?) {
        val frame = mutableMapOf<String, Any?>(
            "kind" to "event",
            "body" to mapOf("encoding" to "utf8", "data" to Json.stringify(message)),
        )
        if (to != null) frame["to"] = to
        write(frame)
    }

    internal fun next(): Any? {
        while (inbox.isEmpty() && !closed) receive()
        return if (inbox.isEmpty()) YonSocket.ENDED else inbox.removeFirst()
    }

    private fun receive() {
        val frame = YonSocket.readFrame()?.let { Json.parse(it) } as? Map<*, *>
        if (frame == null) {
            closed = true
            return
        }
        if (frame["request_id"] != id) kotlin.system.exitProcess(70)
        val body = frame["body"] as? Map<*, *>
        when (frame["kind"]) {
            "message" -> inbox.addLast(Json.parse(body?.get("data")?.toString()))
            "sessions" -> sessions = frame["sessions"] as? List<*> ?: emptyList<Any?>()
            else -> closed = true
        }
    }

    private fun write(fields: Map<String, Any?>) {
        YonSocket.writeFrame(Json.stringify(fields + mapOf("protocol_version" to 1, "request_id" to id)))
    }
}

object YonSocket {
    private const val MAX_FRAME_BYTES = 16 * 1024 * 1024
    internal val ENDED = Any()
    private val input by lazy { java.io.DataInputStream(System.`in`) }
    private val output by lazy { java.io.DataOutputStream(System.out) }

    fun framed(): Boolean = System.getenv("YON_SOCKET_FRAMES") == "1"

    fun readFrame(): String? = try {
        val length = input.readInt()
        if (length < 0 || length > MAX_FRAME_BYTES) kotlin.system.exitProcess(70)
        val payload = ByteArray(length)
        input.readFully(payload)
        String(payload, Charsets.UTF_8)
    } catch (_: java.io.IOException) {
        null
    }

    fun writeFrame(text: String) {
        val payload = text.toByteArray(Charsets.UTF_8)
        output.writeInt(payload.size)
        output.write(payload)
        output.flush()
    }

    fun run(endpoint: YonSocketEndpoint, request: __YonFrame): Any? {
        val session = SocketSession(request.raw()["request_id"].toString())
        attempt(endpoint, session) { endpoint.open(session) }
        while (true) {
            val message = session.next()
            if (message === ENDED) break
            attempt(endpoint, session) { endpoint.message(session, message) }
        }
        try {
            call { endpoint.close(session) }
        } catch (_: Exception) {
        }
        kotlin.system.exitProcess(0)
    }

    private fun call(action: () -> Unit) {
        Yon.renewDeadline()
        action()
    }

    private fun attempt(endpoint: YonSocketEndpoint, session: SocketSession, action: () -> Unit) {
        try {
            call(action)
        } catch (failure: Exception) {
            val event = mapOf(
                "event" to "handler.socket_failed",
                "request_id" to session.id,
                "message" to failure.message.toString(),
            )
            System.err.println(Json.stringify(event))
            try {
                call { endpoint.error(session, failure) }
            } catch (_: Exception) {
            }
        }
    }
}

fun main() {
    Yon.captureStart()
    val raw = if (YonSocket.framed()) YonSocket.readFrame() ?: return else generateSequence(::readLine).joinToString("\n")
    val request = __YonFrame(Json.parse(raw))
    Yon.captureInvocation(request)
    val phase = when (request.raw()["operation"]?.toString()) {
        "middleware.before" -> "before"
        "middleware.after" -> "after"
        "event.deliver" -> "__yonDeliver"
        else -> null
    }
    val entryName = phase ?: request.method()
    val missingStatus = if (phase == null) 405 else if (phase == "__yonDeliver") 404 else 204
    val method = __YON_HANDLER__::class.java.methods.firstOrNull {
        it.name == entryName && it.parameterCount == 1
    } ?: __YON_HANDLER__::class.java.methods.firstOrNull {
        it.name == entryName && it.parameterCount == 0
    }
    val result: Any? = if (method == null) {
        __YonAnswer.empty(missingStatus)
    } else {
        try {
            when {
                method.parameterCount == 0 -> method.invoke(__YON_HANDLER__)
                method.parameterTypes[0] == __YonFrame::class.java -> method.invoke(__YON_HANDLER__, request)
                else -> method.invoke(__YON_HANDLER__, Yon.exchange(request))
            }
        } catch (failed: java.lang.reflect.InvocationTargetException) {
            kotlin.system.exitProcess(1)
        }
    }

    if (result is Sequence<*>) {
        val id = request.raw()["request_id"]?.toString() ?: ""
        try {
            for (event in result) {
                if (!YonStream.event(id, event)) {
                    return
                }
            }
        } catch (_: Throwable) {
            YonStream.failure(id)
        }
        return
    }
    val response = Yon.answered() ?: when {
        result is __YonAnswer -> result
        phase != null -> __YonAnswer.empty(204)
        else -> __YonAnswer.json(Json.stringify(result))
    }
    print(response.envelope())
}
