
@Suppress("UNUSED_PARAMETER")
private fun behaviorBuiltinCache(name: String): Behaviors.CacheAdapter? = null

@Suppress("UNUSED_PARAMETER")
private fun behaviorBuiltinEntities(name: String): Behaviors.EntityAdapter? = null

@Suppress("RedundantSuspendModifier")
private suspend fun behaviorDefaultExchange(request: Map<String, Any?>): Any? = TacYonHttp.send(request)

private object TacYonHttp {
    private const val ORIGIN = "__YON_ORIGIN__"
    private const val FAILED = "The native HTTP request to Yon failed."
    private const val MISSING = "This Android app declares no tac.yon.origin, so its Kotlin HTTP clients have no Yon to call. Set tac.yon.origin in client/manifest.json, or give this client an HTTP adapter in its Setup."
    private const val MOST_BYTES = 1048576
    private const val MOST_PATH_BYTES = 2048
    private const val MOST_URL_BYTES = 8192
    private const val MOST_HEADERS = 128
    private val METHODS = setOf("GET", "HEAD", "POST", "PUT", "PATCH", "DELETE")
    private val REFUSED_HEADERS = setOf("host", "connection", "content-length", "transfer-encoding", "cookie", "origin", "referer")
    private val HEADER_NAME = Regex("[!#$%&'*+.^_`|~0-9A-Za-z-]{1,128}")
    private val PATH_REFUSED = Regex("[\\s\\\\?#%]")
    private val WORKER = java.util.concurrent.Executors.newCachedThreadPool { work ->
        Thread(work, "tachyon-yon-http").apply { isDaemon = true }
    }

    fun send(request: Map<String, Any?>): Any? {
        if (ORIGIN.isEmpty()) throw Behaviors.Failure("HTTP_MISSING", null, MISSING)
        val method = (request["method"] as? String)?.takeIf { it in METHODS } ?: fail("HTTP_REQUEST")
        if (request["redirect"] != "error" || request["credentials"] != "omit") fail("HTTP_REQUEST")
        val limit = (request["maxResponseBytes"] as? Number)?.toInt()?.takeIf { it in 1..MOST_BYTES } ?: fail("HTTP_LIMIT")
        val remaining = ((request["deadline"] as? Number)?.toLong() ?: fail("HTTP_REQUEST")) - System.currentTimeMillis()
        if (remaining <= 0) fail("DEADLINE")
        val address = address(request["path"], request["query"])
        val headers = headers(request["headers"])
        val body = if (request.containsKey("body")) body(method, request["body"]) else null
        val connection = address.openConnection() as java.net.HttpURLConnection
        val running = WORKER.submit<Any?> { exchange(connection, method, headers, body, limit) }
        return try {
            running.get(remaining, java.util.concurrent.TimeUnit.MILLISECONDS)
        } catch (_: java.util.concurrent.TimeoutException) {
            running.cancel(true)
            connection.disconnect()
            fail("DEADLINE")
        } catch (_: InterruptedException) {
            Thread.currentThread().interrupt()
            connection.disconnect()
            fail("CANCELLED")
        } catch (error: java.util.concurrent.ExecutionException) {
            throw error.cause as? Behaviors.Failure ?: Behaviors.Failure("HTTP_NETWORK", null, FAILED)
        }
    }

    private fun fail(code: String): Nothing = throw Behaviors.Failure(code, null, FAILED)

    private fun bytes(text: String): Int = text.toByteArray(Charsets.UTF_8).size

    private fun address(path: Any?, query: Any?): java.net.URL {
        if (path !is String || !path.startsWith("/") || path.startsWith("//") || PATH_REFUSED.containsMatchIn(path)
            || path.split('/').any { it == "." || it == ".." } || bytes(path) > MOST_PATH_BYTES) fail("HTTP_ORIGIN")
        val search = queryText(query ?: emptyMap<String, Any?>())
        val text = ORIGIN + path + if (search.isEmpty()) "" else "?$search"
        if (bytes(text) > MOST_URL_BYTES) fail("HTTP_QUERY")
        return java.net.URL(text)
    }

    private fun queryText(query: Any?): String {
        if (query !is Map<*, *>) fail("HTTP_QUERY")
        return query.entries.joinToString("&") { (key, value) -> queryPair(key, value) }
    }

    private fun queryPair(key: Any?, value: Any?): String {
        if (key !is String || key.length > 256 || !(value is String || value is Number || value is Boolean)) fail("HTTP_QUERY")
        val text = if (value is Double && value % 1.0 == 0.0 && kotlin.math.abs(value) < 1e15) value.toLong().toString() else value.toString()
        if (bytes(text) > 8192) fail("HTTP_QUERY")
        return java.net.URLEncoder.encode(key, "UTF-8") + "=" + java.net.URLEncoder.encode(text, "UTF-8")
    }

    private fun headers(values: Any?): Map<String, String> {
        val declared = values ?: emptyMap<String, Any?>()
        if (declared !is Map<*, *> || declared.size > MOST_HEADERS) fail("HTTP_HEADER")
        return declared.entries.associate { (name, value) -> header(name, value) }
    }

    private fun header(name: Any?, value: Any?): Pair<String, String> {
        val lower = (name as? String)?.lowercase(java.util.Locale.ROOT) ?: fail("HTTP_HEADER")
        if (!HEADER_NAME.matches(lower) || lower in REFUSED_HEADERS || lower.startsWith("sec-") || lower.startsWith("proxy-")
            || value !is String || value.any { it == '\r' || it == '\n' || it == '\u0000' } || bytes(value) > 8192) fail("HTTP_HEADER")
        return lower to value
    }

    private fun body(method: String, value: Any?): ByteArray {
        if (method == "GET" || method == "HEAD") fail("HTTP_BODY")
        val text = try { Behaviors.json(value, MOST_BYTES) } catch (_: RuntimeException) { fail("HTTP_BODY") }
        return text.toByteArray(Charsets.UTF_8)
    }

    private fun exchange(connection: java.net.HttpURLConnection, method: String, headers: Map<String, String>, body: ByteArray?, limit: Int): Any? {
        try {
            connection.requestMethod = method
            connection.instanceFollowRedirects = false
            connection.useCaches = false
            for ((name, value) in headers) connection.setRequestProperty(name, value)
            if (body != null) write(connection, body)
            val status = connection.responseCode
            if (status in 300..399) fail("HTTP_NETWORK")
            if (status !in 200..299) fail("HTTP_STATUS")
            val text = read(connection, limit)
            return if (text.isBlank()) null else parsed(text)
        } catch (_: java.io.IOException) {
            fail("HTTP_NETWORK")
        } finally {
            connection.disconnect()
        }
    }

    private fun write(connection: java.net.HttpURLConnection, body: ByteArray) {
        connection.doOutput = true
        connection.setRequestProperty("content-type", "application/json")
        connection.setFixedLengthStreamingMode(body.size)
        connection.outputStream.use { it.write(body) }
    }

    private fun read(connection: java.net.HttpURLConnection, limit: Int): String {
        if (connection.contentLengthLong > limit) fail("HTTP_LIMIT")
        val output = java.io.ByteArrayOutputStream()
        val buffer = ByteArray(16384)
        connection.inputStream.use { input ->
            while (true) {
                val count = input.read(buffer)
                if (count < 0) break
                if (output.size() + count > limit) fail("HTTP_LIMIT")
                output.write(buffer, 0, count)
            }
        }
        val decoder = Charsets.UTF_8.newDecoder()
            .onMalformedInput(java.nio.charset.CodingErrorAction.REPORT)
            .onUnmappableCharacter(java.nio.charset.CodingErrorAction.REPORT)
        return try { decoder.decode(java.nio.ByteBuffer.wrap(output.toByteArray())).toString() }
            catch (_: java.nio.charset.CharacterCodingException) { fail("HTTP_RESPONSE") }
    }

    private fun parsed(text: String): Any? =
        try { tacParseJson(text) } catch (_: RuntimeException) { fail("HTTP_RESPONSE") }
}
