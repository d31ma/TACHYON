@Setup
class PageProgramSetup {
    companion object {
        fun configure(execution: Behaviors.Execution): Behaviors.Configuration {
            val identity = kotlin.random.Random.nextLong().toULong().toString(16)
            var count = 0
            val program = Behaviors.HttpAdapter { request, _ ->
                val path = request["path"] as? String ?: ""
                if (request["method"] == "POST" && path.endsWith("/increment")) count += 1
                mapOf("identity" to identity, "count" to count)
            }
            return Behaviors.Configuration(http = mapOf("pageprogram" to program))
        }
    }
}
