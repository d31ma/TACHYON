@Store(EntryEntity::class, {"fields":["id","status"],"defaultSize":20,"maxSize":100,"setup":"LedgerSetup"})
interface EntryStore : Repository<EntryEntity, Long> {
    fun findByStatus(status: String): Any?
    fun countByStatus(status: String): Any?
}
