package java.util.concurrent

class ConcurrentHashMap<K, V> {
    private val entries = LinkedHashMap<K, V>()

    fun computeIfAbsent(key: K, create: (K) -> V): V = entries.getOrPut(key) { create(key) }
}

class Semaphore(private var available: Int, fair: Boolean = false) {
    fun tryAcquire(): Boolean {
        if (available <= 0) return false
        available -= 1
        return true
    }

    fun tryAcquire(timeout: Long, unit: TimeUnit): Boolean = tryAcquire()

    fun release() {
        available += 1
    }
}

enum class TimeUnit { NANOSECONDS, MILLISECONDS }
