package java.util

object Collections {
    fun <T> unmodifiableList(items: List<T>): List<T> = items.toList()

    fun <K, V> unmodifiableMap(entries: Map<K, V>): Map<K, V> = entries.toMap()
}

class IdentityHashMap<K, V> {
    private val keys = ArrayList<K>()
    private val values = ArrayList<V>()

    fun put(key: K, value: V): V? {
        val index = keys.indexOfFirst { it === key }
        if (index < 0) {
            keys.add(key)
            values.add(value)
            return null
        }
        val previous = values[index]
        values[index] = value
        return previous
    }

    fun remove(key: K): V? {
        val index = keys.indexOfFirst { it === key }
        if (index < 0) return null
        keys.removeAt(index)
        return values.removeAt(index)
    }
}

class Locale private constructor() {
    companion object {
        val ROOT = Locale()
    }
}
