
class BrokenRule(val field: String, val rule: String)

abstract class YonBuiltInError(name: String, val status: Int, val reference: String) : RuntimeException(name)

class NotSignedIn(reference: String) : YonBuiltInError("NotSignedIn", 401, reference)

class RoleMissing(reference: String) : YonBuiltInError("RoleMissing", 403, reference)

class IdentityCheckFailed(reference: String) : YonBuiltInError("IdentityCheckFailed", 500, reference)

class RequestRejected(
    reference: String,
    val part: String,
    val field: String,
    override val message: String,
) : YonBuiltInError("RequestRejected", 400, reference)

class BodyTooLarge(reference: String, val limitBytes: Long) : YonBuiltInError("BodyTooLarge", 413, reference)

class CheckRejected(
    reference: String,
    val violations: List<BrokenRule>,
    val truncated: Boolean,
) : YonBuiltInError("CheckRejected", 422, reference)

class UnexpectedError(reference: String) : YonBuiltInError("UnexpectedError", 500, reference)

private object __YonBuiltInErrors {
    fun create(value: Any?): RuntimeException {
        val fields = asFields(value)
        val reference = text(fields, "reference")
        return when (text(fields, "type")) {
            "NotSignedIn" -> NotSignedIn(reference)
            "RoleMissing" -> RoleMissing(reference)
            "IdentityCheckFailed" -> IdentityCheckFailed(reference)
            "RequestRejected" -> RequestRejected(reference, text(fields, "part"), text(fields, "field"), text(fields, "message"))
            "BodyTooLarge" -> BodyTooLarge(reference, (fields["limitBytes"] as? Number)?.toLong() ?: 0L)
            "CheckRejected" -> CheckRejected(reference, rules(fields["violations"]), fields["truncated"] == true)
            "UnexpectedError" -> UnexpectedError(reference)
            else -> throw IllegalStateException("Unknown built-in error.")
        }
    }

    private fun asFields(value: Any?): Map<*, *> = value as? Map<*, *> ?: emptyMap<Any?, Any?>()

    private fun text(fields: Map<*, *>, name: String): String = fields[name]?.toString() ?: ""

    private fun rules(value: Any?): List<BrokenRule> =
        (value as? List<*>).orEmpty().map { entry -> asFields(entry).let { rule -> BrokenRule(text(rule, "field"), text(rule, "rule")) } }
}
