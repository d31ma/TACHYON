
private suspend fun behaviorDefaultExchange(request: Map<String, Any?>): Any? = BehaviorBrowserHttp.send(request)

private fun tacThrownCode(error: Throwable): String? = (error as? Behaviors.Failure)?.code

private object BehaviorBrowserHttp {
    private const val FAILED = "The browser HTTP request failed."
    private const val REQUEST_BYTES = 1048576
    private val CODES = setOf(
        "HTTP_REQUEST", "HTTP_ORIGIN", "HTTP_HEADER", "HTTP_QUERY", "HTTP_BODY", "HTTP_LIMIT", "HTTP_RESPONSE",
        "HTTP_STATUS", "HTTP_CAPACITY", "HTTP_NETWORK", "HTTP_MISSING", "CANCELLED", "DEADLINE",
    )

    suspend fun send(request: Map<String, Any?>): Any? {
        val answer = parsed(tacHttpExchange(Behaviors.json(request, REQUEST_BYTES)))
        val error = answer["error"]
        if (error is Map<*, *>) throw Behaviors.Failure(code(error["code"]), null, FAILED)
        if (!answer.containsKey("value")) throw Behaviors.Failure("HTTP_RESPONSE", null, FAILED)
        return answer["value"]
    }

    private fun parsed(text: String): Map<*, *> =
        (try { tacParseJson(text) } catch (_: RuntimeException) { null }) as? Map<*, *>
            ?: throw Behaviors.Failure("HTTP_RESPONSE", null, FAILED)

    private fun code(named: Any?): String = if (named is String && named in CODES) named else "HTTP_NETWORK"
}
