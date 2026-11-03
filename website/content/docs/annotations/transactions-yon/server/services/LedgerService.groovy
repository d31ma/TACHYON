@Service
class LedgerService {
    Behaviors.Context behaviors
    @Needs
    EntryStore entries

    @OnStart
    void open() {
        Behaviors.Options options = new Behaviors.Options("ledger", new Behaviors.Signal())
        behaviors = Behaviors.create("LedgerSetup", options)
    }

    @AllOrNothing({"adapter":"ledger"})
    Object settle(String status) {
        Object open = entries.findByStatus(status)
        Object total = entries.countByStatus(status)
        return java.util.Map.of("open", open, "total", total)
    }
}
