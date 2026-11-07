object TacStore {
    private val sessionValues = HashMap<String, Any?>()

    @JvmStatic
    var preferences: android.content.SharedPreferences? = null

    @JvmStatic
    fun local(key: String, fallback: Any?): Any? {
        val store = preferences ?: return fallback
        return store.all["tachyon.$key"] ?: fallback
    }

    @JvmStatic
    fun setLocal(key: String, value: Any?) {
        val editor = preferences?.edit() ?: return
        when (value) {
            is Int -> editor.putInt("tachyon.$key", value)
            is Long -> editor.putLong("tachyon.$key", value)
            is Boolean -> editor.putBoolean("tachyon.$key", value)
            is Float -> editor.putFloat("tachyon.$key", value)
            else -> editor.putString("tachyon.$key", value?.toString())
        }
        editor.apply()
    }

    @JvmStatic
    fun session(key: String, fallback: Any?): Any? = sessionValues[key] ?: fallback

    @JvmStatic
    fun setSession(key: String, value: Any?) {
        sessionValues[key] = value
    }
}

fun interface TacEmit {
    fun send(payload: String)
}

object TacBridge {
    @JvmStatic
    var emit: TacEmit? = null
}

private object TacAndroidThread {
    var owner: Thread? = null
}

private const val tacPageListeners = false

private const val tacPageSingletons = true

private fun tacPageScope(): TacPageScope = TacPageScope { throw IllegalStateException("DocumentTitle needs a browser page.") }

private fun tacOnOwnerThread(): Boolean = Thread.currentThread() === TacAndroidThread.owner

private fun tacEventFailure() { System.err.println("Tachyon native event delivery failed.") }

private fun tacSend(payload: String) { TacBridge.emit?.send(payload) }

private fun tacFailureAnswer(error: Throwable): String = "{\"error\":\"Native companion operation failed.\"}"

private fun tacIsBigWhole(value: Any?): Boolean = value is java.math.BigInteger

private fun tacQuote(text: String): String = org.json.JSONObject.quote(text)

private fun tacParseJson(text: String): Any? = tacValue(org.json.JSONTokener(text).nextValue())

private fun tacValue(value: Any?): Any? =
    when (value) {
        null, org.json.JSONObject.NULL -> null
        is org.json.JSONArray -> (0 until value.length()).map { tacValue(value.opt(it)) }
        is org.json.JSONObject -> value.keys().asSequence().associateWith { tacValue(value.opt(it)) }
        else -> value
    }

private fun tacPlainModel(value: Any): Any? =
    tacModelFields(value.javaClass).associate { field ->
        field.isAccessible = true
        field.name to tacPlain(field.get(value))
    }

@Synchronized
fun tacNativeInvoke(request: String): String {
    TacAndroidThread.owner = Thread.currentThread()
    return tacInvokeOwned(request)
}
