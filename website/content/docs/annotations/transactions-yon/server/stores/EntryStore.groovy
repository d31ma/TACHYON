@Store(EntryEntity.class, {"fields":["id","status"],"defaultSize":20,"maxSize":100,"setup":"LedgerSetup"})
interface EntryStore {
    Object findByStatus(String status)
    Object countByStatus(String status)
}
