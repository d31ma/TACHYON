@Setup
class OverrideSetup {
    companion object {
        fun configure(execution: Behaviors.Execution): Behaviors.Configuration =
            Behaviors.Configuration(http = mapOf("override" to Behaviors.HttpAdapter { _, _ -> mapOf("message" to "Custom adapter wins") }))
    }
}
