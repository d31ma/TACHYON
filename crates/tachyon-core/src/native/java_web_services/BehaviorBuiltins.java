final class BehaviorBuiltins {
    private static final String INDEXED_DB = "indexeddb";

    private BehaviorBuiltins() {}

    static Behaviors.CacheAdapter cache(String name) {
        return new BrowserCache(name);
    }

    static Behaviors.EntityAdapter entities(String name) {
        return INDEXED_DB.equals(name) ? new BrowserEntities() : null;
    }

    private static final class BrowserCache implements Behaviors.CacheAdapter {
        private final String name;

        BrowserCache(String name) {
            this.name = name;
        }

        public Behaviors.Entry get(String key, Behaviors.Execution execution) {
            Object entry = TacStorage.ask(Behaviors.record("op", "cache.get", "cache", name, "key", key));
            if (!(entry instanceof java.util.Map<?, ?> found) || !Boolean.TRUE.equals(found.get("hit"))) {
                return new Behaviors.Entry();
            }
            Object expiresAt = found.get("expiresAt");
            return new Behaviors.Entry(found.get("value"), expiresAt instanceof Number number ? number.longValue() : 0L);
        }

        public void set(String key, Object value, Behaviors.Write execution) {
            TacStorage.ask(Behaviors.record("op", "cache.set", "cache", name, "key", key, "value", value,
                    "expiresAt", execution.expiresAt, "maxEntries", execution.policy.maxEntries));
        }

        public void delete(String key, Behaviors.Execution execution) {
            TacStorage.ask(Behaviors.record("op", "cache.delete", "cache", name, "key", key));
        }
    }

    private static final class BrowserEntities implements Behaviors.EntityAdapter {
        public Object execute(java.util.Map<String, Object> command, Behaviors.Execution execution) {
            return TacStorage.ask(Behaviors.record("op", "entity", "command", command));
        }
    }
}
