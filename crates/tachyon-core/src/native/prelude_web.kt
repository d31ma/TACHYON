@JsFun("(payload) => globalThis[Symbol.for('tachyon.kotlin.host')].emit(payload)")
private external fun tacHostEmit(payload: String)

@JsFun("(text) => globalThis[Symbol.for('tachyon.kotlin.host')].web(text)")
private external fun tacHostWeb(text: String): String

private const val tacPageListeners = true

private fun tacOnOwnerThread(): Boolean = true

private fun tacEventFailure() { println("Tachyon web event delivery failed.") }

private fun tacSend(payload: String) { tacHostEmit(payload) }

private fun tacFailureAnswer(error: Throwable): String =
    "{\"error\":${tacQuote(error.message ?: "Kotlin service operation failed.")}}"

private fun tacIsBigWhole(value: Any?): Boolean = false

private fun tacQuote(text: String): String {
    val quoted = StringBuilder("\"")
    for (character in text) {
        when (character) {
            '"' -> quoted.append("\\\"")
            '\\' -> quoted.append("\\\\")
            '\n' -> quoted.append("\\n")
            '\r' -> quoted.append("\\r")
            '\t' -> quoted.append("\\t")
            '\b' -> quoted.append("\\b")
            '\u000C' -> quoted.append("\\f")
            else -> if (character < ' ') quoted.append("\\u" + character.code.toString(16).padStart(4, '0')) else quoted.append(character)
        }
    }
    return quoted.append('"').toString()
}

private fun tacParseJson(text: String): Any? = TacJsonReader(text).document()

private class TacJsonReader(private val text: String) {
    private var at = 0

    fun document(): Any? {
        val value = value(0)
        space()
        require(at == text.length) { "Unexpected text after the JSON value." }
        return value
    }

    private fun space() {
        while (at < text.length && text[at] in " \t\r\n") at++
    }

    private fun value(depth: Int): Any? {
        require(depth <= 64) { "JSON is nested too deeply." }
        space()
        require(at < text.length) { "JSON ended early." }
        return when (text[at]) {
            '{' -> objectValue(depth)
            '[' -> listValue(depth)
            '"' -> stringValue()
            't' -> word("true", true)
            'f' -> word("false", false)
            'n' -> word("null", null)
            else -> numberValue()
        }
    }

    private fun word(expected: String, value: Any?): Any? {
        require(text.startsWith(expected, at)) { "Unknown JSON word." }
        at += expected.length
        return value
    }

    private fun objectValue(depth: Int): Map<String, Any?> {
        val entries = LinkedHashMap<String, Any?>()
        at++
        space()
        if (at < text.length && text[at] == '}') { at++; return entries }
        while (true) {
            space()
            require(at < text.length && text[at] == '"') { "A JSON key must be text." }
            val key = stringValue()
            space()
            require(at < text.length && text[at] == ':') { "A JSON key needs a colon." }
            at++
            entries[key] = value(depth + 1)
            space()
            require(at < text.length) { "JSON ended early." }
            if (text[at] == '}') { at++; return entries }
            require(text[at] == ',') { "JSON entries need commas." }
            at++
        }
    }

    private fun listValue(depth: Int): List<Any?> {
        val items = ArrayList<Any?>()
        at++
        space()
        if (at < text.length && text[at] == ']') { at++; return items }
        while (true) {
            items.add(value(depth + 1))
            space()
            require(at < text.length) { "JSON ended early." }
            if (text[at] == ']') { at++; return items }
            require(text[at] == ',') { "JSON items need commas." }
            at++
        }
    }

    private fun stringValue(): String {
        val result = StringBuilder()
        at++
        while (true) {
            require(at < text.length) { "JSON text is not closed." }
            val character = text[at++]
            when (character) {
                '"' -> return result.toString()
                '\\' -> result.append(escaped())
                else -> result.append(character)
            }
        }
    }

    private fun escaped(): Char {
        require(at < text.length) { "JSON text is not closed." }
        return when (val code = text[at++]) {
            'n' -> '\n'
            'r' -> '\r'
            't' -> '\t'
            'b' -> '\b'
            'f' -> '\u000C'
            'u' -> {
                require(at + 4 <= text.length) { "A JSON character code is too short." }
                val hex = text.substring(at, at + 4)
                at += 4
                hex.toInt(16).toChar()
            }
            else -> code
        }
    }

    private fun numberValue(): Any {
        val start = at
        while (at < text.length && text[at] in "+-0123456789.eE") at++
        val number = text.substring(start, at)
        require(number.isNotEmpty()) { "Unknown JSON value." }
        if (number.any { it in ".eE" }) return number.toDouble()
        return number.toIntOrNull() ?: number.toLongOrNull() ?: number.toDouble()
    }
}

private fun tacModelEntries(value: Any?): Map<*, *> =
    value as? Map<*, *> ?: throw IllegalArgumentException("The event payload is not a model.")

object TacWebHost {
    private const val NO_PAGE = "This browser API needs the page, and this service runs without one."

    fun call(api: String, call: String, arguments: List<Any?> = emptyList()): Any? {
        val request = "{\"api\":${tacQuote(api)},\"call\":${tacQuote(call)},\"args\":${tacEncode(arguments)}}"
        val answer = tacObject(tacParseJson(tacHostWeb(request))) ?: throw IllegalStateException(NO_PAGE)
        val error = answer["error"]
        if (error != null) throw IllegalStateException(error.toString())
        return answer["value"]
    }
}

@OptIn(kotlin.js.ExperimentalJsExport::class)
@JsExport
fun tacInvoke(request: String): String = tacInvokeOwned(request)
