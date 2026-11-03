@Delegate
class LedgerAdapterDelegate {
    companion object : Behaviors.EntityAdapter {
    private const val TRANSACTION = "transaction"
    private const val KIND = "kind"
    private const val OPERATION = "operation"
    val commands = mutableListOf<String>()
    private var sequence = 0

    override fun execute(command: Map<String, Any?>, execution: Behaviors.Execution): Any? {
        val identifier = command[TRANSACTION]
        if (command[OPERATION] == TRANSACTION) {
            var entry = command["action"].toString()
            if (identifier != null) entry = entry + ":" + identifier
            commands.add(entry)
            if (command["action"] != "begin") return mapOf("ok" to true)
            sequence++
            return mapOf(TRANSACTION to "t" + sequence)
        }
        if (command[OPERATION] == "save") return saved(command, identifier)
        val query = command["query"] as Map<*, *>
        val transaction = identifier ?: "none"
        commands.add(query[KIND].toString() + "@" + transaction)
        if (query[KIND] == "count") return 1L
        return listOf(mapOf("id" to 1L, "status" to "open"))
    }

    private fun saved(command: Map<String, Any?>, identifier: Any?): Any? {
        commands.add("save@" + identifier)
        return command
    }
}
}
