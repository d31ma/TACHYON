@Setup
class CatalogSetup {
    static final java.util.List<java.util.Map<String, Object>> ROWS = java.util.List.of(
        java.util.Map.of("id", 1, "status", "active"),
        java.util.Map.of("id", 2, "status", "retired"))

    static Behaviors.Configuration configure(Behaviors.Execution execution) {
        Behaviors.EntityAdapter catalog = (command, call) -> {
            java.util.Map<String, Object> query = Behaviors.map(command.get("query"))
            java.util.List<?> conditions = (java.util.List<?>) query.get("where")
            java.util.Map<String, Object> condition = Behaviors.map(conditions.get(0))
            boolean keepUnequalRows = condition.get("operator").equals("ne")
            java.util.List<java.util.Map<String, Object>> matchingRows = new java.util.ArrayList<>()
            for (java.util.Map<String, Object> row : ROWS) {
                boolean isEqual = row.get(condition.get("field")).equals(condition.get("value"))
                boolean isMatch = keepUnequalRows ? !isEqual : isEqual
                if (isMatch) matchingRows.add(row)
            }
            if (query.get("kind").equals("count")) return (long) matchingRows.size()
            return matchingRows
        }
        return new Behaviors.Configuration(java.util.Map.of(), java.util.Map.of(),
            java.util.Map.of("catalog", catalog), null,
            (command, call) -> command.get("entity").equals("ProductEntity"))
    }
}
