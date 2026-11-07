
fun tacInt(value: Any?): Int =
    when (value) {
        is Number -> value.toInt()
        is Boolean -> if (value) 1 else 0
        else -> value?.toString()?.toDoubleOrNull()?.toInt() ?: 0
    }

fun tacLong(value: Any?): Long =
    when (value) {
        is Number -> value.toLong()
        is Boolean -> if (value) 1L else 0L
        else -> value?.toString()?.toDoubleOrNull()?.toLong() ?: 0L
    }

fun tacDouble(value: Any?): Double =
    when (value) {
        is Number -> value.toDouble()
        is Boolean -> if (value) 1.0 else 0.0
        else -> value?.toString()?.toDoubleOrNull() ?: 0.0
    }

fun tacBoolean(value: Any?): Boolean =
    when (value) {
        is Boolean -> value
        is Number -> value.toDouble() != 0.0
        is String -> value.isNotEmpty()
        else -> false
    }

fun tacString(value: Any?): String = value?.toString() ?: ""

inline fun <reified T> tacRead(value: Any?): T = tacReadType(kotlin.reflect.typeOf<T>(), value) as T

fun tacReadType(type: kotlin.reflect.KType, value: Any?): Any? {
    if (value == null && type.isMarkedNullable) return null
    val item = type.arguments.firstOrNull()?.type
    return when (val kind = type.classifier) {
        Int::class -> tacInt(value)
        Long::class -> tacLong(value)
        Double::class -> tacDouble(value)
        Float::class -> tacDouble(value).toFloat()
        Boolean::class -> tacBoolean(value)
        String::class -> tacString(value)
        List::class, MutableList::class, Collection::class, Iterable::class ->
            (value as? List<*> ?: emptyList<Any?>()).map { if (item == null) it else tacReadType(item, it) }
        Any::class, Map::class, MutableMap::class -> value
        is kotlin.reflect.KClass<*> -> tacModel(kind, value)
        else -> value
    }
}
