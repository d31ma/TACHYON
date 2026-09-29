@Service
class LedgerService {
    lateinit var behaviors: Behaviors.Context
    @Needs
    val entries: EntryStore

    @OnStart
    fun open() {
        val options = Behaviors.Options("ledger", Behaviors.Signal())
        behaviors = Behaviors.create("LedgerSetup", options)
    }

    @AllOrNothing({"adapter":"ledger"})
    fun settle(status: String): Any? {
        val open = entries.findByStatus(status)
        val total = entries.countByStatus(status)
        return mapOf("open" to open, "total" to total)
    }
}
