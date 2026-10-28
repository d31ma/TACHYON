@Setup
class LedgerSetup {
    companion object {
        fun configure(execution: Behaviors.Execution): Behaviors.Configuration = Behaviors.Configuration(
            entities = mapOf("ledger" to LedgerAdapterDelegate),
            authorizeEntity = Behaviors.Authorization { _, _ -> true })
    }
}
