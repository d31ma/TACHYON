object BrowserFakes {
    private const val MISSING = "This browser API needs the page. A test unit has no page: give the call an answer with BrowserFakes.answer in the test unit."
    private val answers = mutableMapOf<String, Any?>()

    fun answer(api: String, call: String, value: Any?) {
        answers["$api.$call"] = value
    }

    fun reply(api: String, call: String): Any? {
        val key = "$api.$call"
        check(answers.containsKey(key)) { MISSING }
        return answers[key]
    }
}

object TacWebHost {
    @Suppress("UNUSED_PARAMETER")
    fun call(api: String, call: String, arguments: List<Any?> = emptyList()): Any? = BrowserFakes.reply(api, call)
}
