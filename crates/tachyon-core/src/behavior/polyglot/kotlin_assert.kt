
object Assert {
    private const val NOTHING_THROWN = "Expected an error to be thrown, but nothing was thrown."
    private const val MESSAGE_SEPARATOR = " ==> "

    fun equals(expected: Any?, actual: Any?, message: String? = null) {
        confirm(same(expected, actual), "expected: <$expected> but was: <$actual>", message)
    }

    fun isTrue(condition: Boolean, message: String? = null) {
        confirm(condition, "expected: <true> but was: <false>", message)
    }

    fun throws(action: () -> Any?): Throwable = throws(action, null)

    fun throws(action: () -> Any?, message: String?): Throwable {
        val thrown = thrownBy(action)
        confirm(thrown != null, NOTHING_THROWN, message)
        return thrown!!
    }

    private fun thrownBy(action: () -> Any?): Throwable? {
        try {
            action()
            return null
        } catch (error: Throwable) {
            return error
        }
    }

    private fun same(expected: Any?, actual: Any?): Boolean {
        if (isWhole(expected) && isWhole(actual)) {
            return (expected as Number).toLong() == (actual as Number).toLong()
        }
        return expected == actual
    }

    private fun isWhole(value: Any?): Boolean = value is Byte || value is Short || value is Int || value is Long

    private fun confirm(passed: Boolean, failure: String, message: String?) {
        if (!passed) {
            throw AssertionError(text(failure, message))
        }
    }

    private fun text(failure: String, message: String?): String {
        if (message == null) {
            return failure
        }
        return "$message$MESSAGE_SEPARATOR$failure"
    }
}
