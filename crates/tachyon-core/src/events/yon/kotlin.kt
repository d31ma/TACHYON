class ApplicationEventPublisher {
    fun <T : Any> publishEvent(value: T, retain: Boolean = false): T = __YonEvents.publishModel(value, retain)
}
private class __YonEvents(vararg topics: String) : AutoCloseable {
    companion object {
        private val active = ThreadLocal<__YonEvents>()
        fun delivery(body: String, vararg topics: String): __YonEvents {
            val events = __YonEvents(*topics)
            val delivery = Json.parse(body) as Map<*, *>
            events.only = (delivery["listeners"] as List<*>).map { it.toString() }.toSet()
            events.delivered = delivery["topic"].toString() to Json.stringify(delivery["payload"])
            return events
        }
        fun <T> publish(name: String, value: T, retain: Boolean): T {
            try {
            val scope = active.get() ?: error("Local event owner is disposed")
            check(!scope.closed) { "Local event owner is disposed" }
            val json = Json.stringify(value)
            check(json.toByteArray(Charsets.UTF_8).size <= 65536 && scope.queue.size < 128) { "Local event capacity exceeded" }
            if (retain) scope.retained[name] = json
            scope.queue.add(name to json)
            if (name in scope.topics) scope.outbound.add(name to json)
            } catch (failure: Exception) { System.err.println("Yon local event publication failed.") }
            return value
        }
        fun <T : Any> publishModel(value: T, retain: Boolean): T {
            val plain = value is Map<*, *> || value is Iterable<*> || value is CharSequence || value is Number || value is Boolean || value is Enum<*> || value.javaClass.isArray
            if (plain) {
                System.err.println("Yon local event publication failed.")
                return value
            }
            return publish(value.javaClass.simpleName, value, retain)
        }
        fun fits(models: Map<*, *>, shape: String, value: Any?): Boolean {
            if (shape.startsWith("?")) return value == null || fits(models, shape.substring(1), value)
            if (shape.startsWith("[]")) return value is List<*> && value.all { fits(models, shape.substring(2), it) }
            when (shape) {
                "any" -> return true
                "string" -> return value is String
                "integer" -> return value is Long || value is Int || value is java.math.BigInteger
                "number" -> return value is Number
                "boolean" -> return value is Boolean
            }
            val model = models[shape] as? Map<*, *> ?: return false
            val objectValue = value as? Map<*, *> ?: return false
            val fields = model["fields"] as Map<*, *>
            if (model["open"] != true && objectValue.keys.any { !fields.containsKey(it) }) return false
            for ((name, declared) in fields) {
                val field = declared.toString()
                val required = field.startsWith("!")
                if (!objectValue.containsKey(name)) {
                    if (required) return false
                    continue
                }
                if (!fits(models, if (required) field.substring(1) else field, objectValue[name])) return false
            }
            return true
        }
        fun matches(matcher: Any?, value: Any?, event: String): Boolean =
            try { test(matcher as Map<*, *>, value, event) } catch (failure: Exception) { false }
        private fun operand(node: Map<*, *>, value: Any?, event: String): Any? {
            when (node["k"]) { "lit" -> return node["v"]; "event" -> return event }
            var current = value
            for (segment in (node["path"] as? List<*>) ?: emptyList<Any?>()) {
                current = when {
                    current is Map<*, *> -> current[segment.toString()]
                    current is List<*> && segment is Number && segment.toInt() in current.indices -> current[segment.toInt()]
                    else -> return null
                }
            }
            return current
        }
        private fun truthy(value: Any?): Boolean = when (value) {
            null -> false
            is Boolean -> value
            is Number -> value.toDouble() != 0.0
            is String -> value.isNotEmpty()
            else -> true
        }
        private fun test(node: Map<*, *>, value: Any?, event: String): Boolean = when (node["k"]) {
            "and" -> test(node["l"] as Map<*, *>, value, event) && test(node["r"] as Map<*, *>, value, event)
            "or" -> test(node["l"] as Map<*, *>, value, event) || test(node["r"] as Map<*, *>, value, event)
            "not" -> !test(node["e"] as Map<*, *>, value, event)
            "truthy" -> truthy(operand(node["e"] as Map<*, *>, value, event))
            "cmp" -> compare(node["op"].toString(), operand(node["l"] as Map<*, *>, value, event), operand(node["r"] as Map<*, *>, value, event))
            else -> false
        }
        private fun compare(op: String, left: Any?, right: Any?): Boolean {
            val numbers = left is Number && right is Number
            val strings = left is String && right is String
            if (op == "eq" || op == "ne") {
                val equal = when {
                    numbers -> (left as Number).toDouble() == (right as Number).toDouble()
                    strings || left is Boolean -> left == right
                    else -> left == null && right == null
                }
                return (op == "eq") == equal
            }
            val order = when {
                numbers -> (left as Number).toDouble().compareTo((right as Number).toDouble())
                strings -> (left as String).compareTo(right as String)
                else -> return false
            }
            return when (op) { "lt" -> order < 0; "le" -> order <= 0; "gt" -> order > 0; else -> order >= 0 }
        }
    }
    private class Subscription(val name: String, val callback: (Any?) -> Any?, val order: Long, val condition: Any?, val models: Map<*, *>?, val returns: String?, val sequence: Int)
    private val queue = java.util.ArrayDeque<Pair<String,String>>()
    private val retained = linkedMapOf<String,String>()
    private val listeners = mutableListOf<Subscription>()
    val destroyers = mutableListOf<() -> Unit>()
    private val outbound = mutableListOf<Pair<String,String>>()
    private val topics = topics.toSet()
    private var only: Set<String>? = null
    private var delivered: Pair<String,String>? = null
    private var closed = false
    private var deliveries = 0
    init { check(active.get() == null) { "Nested local event invocation" }; active.set(this) }
    fun receiveDelivery() { delivered?.let { queue.add(it) } }
    fun listen(options: String, callback: (Any?) -> Any?, replay: Boolean, key: String) {
        if (only?.contains(key) == false) return
        val compiled = Json.parse(options) as Map<*, *>
        val order = (compiled["order"] as? Number)?.toLong() ?: 0L
        val models = compiled["models"] as? Map<*, *>
        val returns = compiled["returns"] as? String
        for (name in compiled["events"] as List<*>) {
            check(listeners.size < 1024) { "Local listener capacity exceeded" }
            listeners.add(Subscription(name.toString(), callback, order, compiled["condition"], models, returns, listeners.size))
            if (replay) retained[name.toString()]?.let { queue.add(name.toString() to it) }
        }
        listeners.sortWith(compareBy<Subscription>({ it.order }, { it.sequence }))
    }
    private fun republish(result: Any?, returns: String?) {
        if (returns == null) return
        val items = if (result is List<*>) result.take(32) else listOf(result)
        for (item in items) {
            if (item != null) publish(returns, item, false)
        }
    }
    fun drain() {
        while (!queue.isEmpty()) {
            val (name,json) = queue.remove()
            if (++deliveries > 4096) { queue.clear(); System.err.println("Yon local event delivery bound exceeded."); return }
            for (listener in listeners) if (listener.name == name) {
                val value = Json.parse(json)
                if (listener.models != null && !fits(listener.models, name, value)) {
                    System.err.println("Yon local event payload does not fit the listener model.")
                    continue
                }
                if (listener.condition != null && !matches(listener.condition, value, name)) continue
                try { republish(listener.callback(value), listener.returns) } catch (failure: Exception) { System.err.println("Yon local event listener failed.") }
            }
        }
        sendTopics()
    }
    private fun sendTopics() {
        for ((name, json) in outbound) {
            try { YonManagedResources.call(mapOf("operation" to "topic", "topic" to name, "payload_json" to json)) }
            catch (failure: Exception) { System.err.println("Yon topic publication failed.") }
        }
        outbound.clear()
    }
    override fun close() {
        if (closed) return
        closed = true; queue.clear(); listeners.clear()
        for (destroy in destroyers.asReversed()) {
            try { destroy() } catch (failure: Exception) { System.err.println("Yon local lifecycle cleanup failed.") }
        }
        retained.clear(); active.remove()
    }
}
