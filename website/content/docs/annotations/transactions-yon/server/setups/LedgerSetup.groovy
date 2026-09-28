@Setup
class LedgerSetup {
    static Behaviors.Configuration configure(Behaviors.Execution execution) {
        return new Behaviors.Configuration(java.util.Map.of(), java.util.Map.of(),
            java.util.Map.of("ledger", new LedgerAdapterHelper()), null, (command, current) -> true)
    }
}
