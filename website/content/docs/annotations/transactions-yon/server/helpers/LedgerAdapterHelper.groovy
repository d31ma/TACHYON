@Helper
class LedgerAdapterHelper implements Behaviors.EntityAdapter {
    private static final String TRANSACTION = "transaction"
    private static final String ACTION = "action"
    private static final String KIND = "kind"
    static final java.util.List<String> commands = new java.util.ArrayList<>()
    static int sequence = 0

    public Object execute(java.util.Map<String, Object> command, Behaviors.Execution execution) {
        Object identifier = command.get(TRANSACTION)
        if (command.get("operation").equals(TRANSACTION)) {
            String entry = String.valueOf(command.get(ACTION))
            if (identifier != null) entry = entry + ":" + identifier
            commands.add(entry)
            if (!command.get(ACTION).equals("begin")) return java.util.Map.of("ok", true)
            sequence++
            return java.util.Map.of(TRANSACTION, "t" + sequence)
        }
        java.util.Map<String, Object> query = Behaviors.map(command.get("query"))
        Object transaction = identifier
        if (transaction == null) transaction = "none"
        commands.add(query.get(KIND) + "@" + transaction)
        if (query.get(KIND).equals("count")) return 1L
        return java.util.List.of(java.util.Map.of("id", 1L, "status", "open"))
    }
}
