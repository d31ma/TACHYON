@Store(EntryEntity.class, {"fields":["id","status"],"defaultSize":20,"maxSize":100})
class EntryStore {
    @Needs
    Behaviors.Context behaviors
    Object findByStatus(String status) {}
    Object countByStatus(String status) {}

    @AllOrNothing({"adapter":"ledger"})
    Object settle(String status) {
        Object open = findByStatus(status)
        Object total = countByStatus(status)
        return java.util.Map.of("open", open, "total", total)
    }
}
