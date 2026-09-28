import kotlin.math.pow

annotation class Volatile

annotation class Synchronized

inline fun <R> synchronized(lock: Any, block: () -> R): R = block()

class InterruptedException : Exception()

class TacCharset

object Charsets {
    val UTF_8 = TacCharset()
}

fun String.toByteArray(charset: TacCharset = Charsets.UTF_8): ByteArray = encodeToByteArray()

fun String.lowercase(locale: java.util.Locale): String = lowercase()

fun String.uppercase(locale: java.util.Locale): String = uppercase()

fun String.codePoints(): TacCodePoints = TacCodePoints(this)

class TacCodePoints(private val text: String) {
    fun allMatch(test: (Int) -> Boolean): Boolean {
        var index = 0
        while (index < text.length) {
            val first = text[index++]
            val paired = first.isHighSurrogate() && index < text.length && text[index].isLowSurrogate()
            val point = if (paired) 0x10000 + ((first.code - 0xD800) shl 10) + (text[index++].code - 0xDC00) else first.code
            if (!test(point)) return false
        }
        return true
    }
}

fun <K : Comparable<K>, V> sortedMapOf(): MutableMap<K, V> = TacSortedMap()

class TacSortedMap<K : Comparable<K>, V> : AbstractMutableMap<K, V>() {
    private val backing = LinkedHashMap<K, V>()

    override val entries: MutableSet<MutableMap.MutableEntry<K, V>>
        get() = backing.entries.sortedBy { it.key }.toMutableSet()

    override fun put(key: K, value: V): V? = backing.put(key, value)
}

object Character {
    fun isHighSurrogate(character: Char): Boolean = character.isHighSurrogate()

    fun isLowSurrogate(character: Char): Boolean = character.isLowSurrogate()
}

object Math {
    fun rint(value: Double): Double = kotlin.math.round(value)

    fun round(value: Double): Long = kotlin.math.floor(value + 0.5).toLong()

    fun pow(base: Double, exponent: Double): Double = base.pow(exponent)

    fun random(): Double = kotlin.random.Random.nextDouble()
}

@JsFun("() => Date.now()")
private external fun tacEpochMilliseconds(): Double

object System {
    private val started = kotlin.time.TimeSource.Monotonic.markNow()

    fun currentTimeMillis(): Long = tacEpochMilliseconds().toLong()

    fun nanoTime(): Long = started.elapsedNow().inWholeNanoseconds
}

object Thread {
    fun sleep(milliseconds: Long) {
        val started = kotlin.time.TimeSource.Monotonic.markNow()
        while (started.elapsedNow().inWholeMilliseconds < milliseconds) {}
    }

    fun currentThread(): Thread = this

    fun interrupt() = Unit
}
