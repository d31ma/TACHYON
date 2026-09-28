@Setup
class LedgerSetup {
    companion object {
        fun configure(execution: Behaviors.Execution): Behaviors.Configuration = Behaviors.Configuration(
            entities = mapOf("ledger" to LedgerAdapterHelper),
            authorizeEntity = Behaviors.Authorization { _, _ -> true })
    }
}
