@Setup()
class RuntimeAdaptersSetup {
    private static final String QUERY = "query";
    private static final String OPERATION = "operation";
    static final java.util.Map<String, Behaviors.Entry> values = new java.util.HashMap<>();
    static int authorized = 0;
    static int calls = 0;
    static int reads = 0;
    static int writes = 0;
    static int deletes = 0;
    static String operation = "";
    static int configured = 0;
    static boolean allow = true;
    static boolean cacheUnavailable = false;
    static Behaviors.Configuration configure(Behaviors.Execution execution) {
        configured++;
        execution.check();
        return new Behaviors.Configuration(
            java.util.Map.of("catalog", catalogCache()),
            java.util.Map.of("api", (request, current) -> checkHttpRequest(request, current)),
            java.util.Map.of("data", (command, current) -> runEntityCommand(command, current)),
            (call, current) -> authorizeCache(call, current),
            (command, current) -> authorizeEntity(current));
    }
    static Behaviors.CacheAdapter catalogCache() {
        return new Behaviors.CacheAdapter() {
            public Behaviors.Entry get(String key, Behaviors.Execution execution) {
                execution.check();
                reads++;
                failWhenCacheUnavailable();
                return values.getOrDefault(key, new Behaviors.Entry());
            }
            public void set(String key, Object value, Behaviors.Write write) {
                write.execution.check();
                failWhenCacheUnavailable();
                writes++;
                values.put(key, new Behaviors.Entry(value, write.expiresAt));
            }
            public void delete(String key, Behaviors.Execution execution) {
                execution.check();
                failWhenCacheUnavailable();
                deletes++;
                values.remove(key);
            }
        };
    }
    static void failWhenCacheUnavailable() {
        if (cacheUnavailable) {
            throw new IllegalStateException("private adapter error");
        }
    }
    static Object checkHttpRequest(java.util.Map<String, Object> request, Behaviors.Execution current) {
        current.check();
        boolean followsPolicy = request.get("redirect").equals("error") && request.get("credentials").equals("omit");
        if (!followsPolicy) {
            throw new AssertionError("HTTP policy missing");
        }
        return request;
    }
    static Object runEntityCommand(java.util.Map<String, Object> command, Behaviors.Execution current) {
        current.check();
        if (!command.get(OPERATION).equals(QUERY)) {
            return command;
        }
        String kind = (String)Behaviors.map(command.get(QUERY)).get("kind");
        if (kind.equals("count")) {
            return 1;
        }
        if (kind.equals("exists")) {
            return true;
        }
        return java.util.List.of(java.util.Map.of("id", "one", QUERY, command.get(QUERY)));
    }
    static boolean authorizeCache(java.util.Map<String, Object> call, Behaviors.Execution current) {
        current.check();
        operation = (String)call.get(OPERATION);
        if (!java.util.Set.of("cache:read", "cache:evict", "cache:put").contains(operation)) {
            throw new AssertionError("authorization operation");
        }
        authorized++;
        return allow;
    }
    static boolean authorizeEntity(Behaviors.Execution current) {
        current.check();
        return allow;
    }
}

