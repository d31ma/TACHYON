@Store(EntryEntity::class, {"fields":["id","status"],"defaultSize":20,"maxSize":100})
class EntryStore {
    @Needs
    val behaviors: Behaviors.Context
    fun findByStatus(status: String): Any? {}
    fun countByStatus(status: String): Any? {}

    @AllOrNothing({"adapter":"ledger"})
    fun settle(status: String): Any? {
        val open = findByStatus(status)
        val total = countByStatus(status)
        return mapOf("open" to open, "total" to total)
    }
}
