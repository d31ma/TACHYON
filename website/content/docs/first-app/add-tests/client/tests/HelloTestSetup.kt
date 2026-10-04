@TestSetup
class HelloTestSetup {
  companion object {
    fun configure(execution: Behaviors.Execution): Behaviors.Configuration =
      Behaviors.Configuration(http = mapOf("greeting" to Behaviors.HttpAdapter { _, _ -> mapOf("message" to "Hi") }))
  }
}
