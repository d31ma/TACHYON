@Setup
class OrdersSetup {
    companion object {
        fun configure(execution: Behaviors.Execution): Behaviors.Configuration =
            Behaviors.Configuration(caches = mapOf(), authorize = Behaviors.Authorization { _, _ -> true })
    }
}
