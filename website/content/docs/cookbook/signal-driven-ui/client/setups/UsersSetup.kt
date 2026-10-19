@Setup
class UsersSetup {
    companion object {
        fun configure(execution: Behaviors.Execution): Behaviors.Configuration {
            val saved = mutableListOf<Map<String, Any?>>()
            val users = Behaviors.HttpAdapter { request, _ ->
                if (request["method"] == "GET") return@HttpAdapter saved.toList()
                val user = mapOf("name" to request["body"])
                saved.add(user)
                user
            }
            return Behaviors.Configuration(http = mapOf("users" to users))
        }
    }
}
