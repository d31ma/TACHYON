final class Behaviors {
    private Behaviors() {}
    static final String MODULE = "__MODULE__";
    static final String ENVIRONMENT = "__ENVIRONMENT__";
    static final class Failure extends RuntimeException {
        final String code;
        final String path;
        Failure(String code) { this(code, null); }
        Failure(String code, String path) { this(code, path, code); }
        Failure(String code, String path, String message) { super(message); this.code = code; this.path = path; }
    }
    static Failure fail(String code) { return new Failure(code); }
    static final String WRONG_SHAPE = "The server answer does not have the shape of the model (HTTP_RESPONSE).";
    static <T> T answer(Object value, String shape, java.util.function.Function<Object, T> read) {
        boolean object = value instanceof java.util.Map<?, ?>;
        boolean list = value instanceof java.util.List<?>;
        boolean fits = value == null || (shape.equals("object") == object && shape.equals("list") == list);
        if (!fits) throw new Failure("HTTP_RESPONSE", null, WRONG_SHAPE);
        try { return read.apply(value); } catch (RuntimeException error) { throw new Failure("HTTP_RESPONSE", null, WRONG_SHAPE); }
    }
    static final class Signal {
        private volatile boolean aborted;
        private final Signal parent;
        private final Context owner;
        Signal() { this(null, null); }
        private Signal(Signal parent, Context owner) { this.parent = parent; this.owner = owner; }
        void abort() { aborted = true; }
        boolean aborted() { return aborted || (parent != null && parent.aborted()) || (owner != null && owner.closed); }
    }
    static final class Options {
        final String partition;
        final Signal signal;
        final long timeoutMs;
        final java.util.List<String> profiles;
        final java.util.Map<String, String> properties;
        Options(String partition, Signal signal, long timeoutMs) {
            this(partition, signal, timeoutMs, java.util.List.of(), java.util.Map.of());
        }
        Options(String partition, Signal signal, long timeoutMs, java.util.List<String> profiles, java.util.Map<?, ?> properties) {
            if (partition == null || partition.isBlank() || bytes(partition) > 256 || signal == null || timeoutMs < 1 || timeoutMs > 60000) throw fail("configuration");
            this.partition = partition; this.signal = signal; this.timeoutMs = timeoutMs;
            if (profiles == null || profiles.size() > 32) throw fail("configuration");
            java.util.Set<String> unique = new java.util.HashSet<>();
            for (Object profile : profiles) if (!(profile instanceof String) || !((String)profile).matches("[A-Za-z_$][A-Za-z0-9_$]{0,63}") || !unique.add((String)profile)) throw fail("configuration");
            this.profiles = java.util.List.copyOf(profiles); this.properties = propertyInputs(properties);
        }
        Options(String partition, Signal signal) { this(partition, signal, 10000); }
    }
    static final class Execution {
        final String environment = ENVIRONMENT;
        final String partition;
        final Signal signal;
        final long deadline;
        final Context owner;
        final long monotonicDeadline;
        Execution(Options options, Context owner) {
            partition = options.partition; signal = new Signal(options.signal, owner); this.owner = owner;
            deadline = System.currentTimeMillis() + options.timeoutMs;
            monotonicDeadline = System.nanoTime() + options.timeoutMs * 1000000;
        }
        void check() {
            if (signal.aborted() || (owner != null && owner.closed)) throw fail("aborted");
            if (System.nanoTime() - monotonicDeadline >= 0) throw fail("timeout");
        }
    }
    interface CacheAdapter {
        Entry get(String key, Execution execution);
        void set(String key, Object value, Write execution);
        void delete(String key, Execution execution);
    }
    interface HttpAdapter { Object exchange(java.util.Map<String, Object> request, Execution execution); }
    interface EntityAdapter { Object execute(java.util.Map<String, Object> command, Execution execution); }
    interface Authorization { boolean allow(java.util.Map<String, Object> call, Execution execution); }
    interface Call { Object run(); }
    static final class Retry {
        final int maxRetries, delayMs, maxDelayMs, jitterMs;
        final double multiplier;
        Retry(int maxRetries, int delayMs, double multiplier, int maxDelayMs, int jitterMs) {
            this.maxRetries = maxRetries; this.delayMs = delayMs; this.multiplier = multiplier; this.maxDelayMs = maxDelayMs; this.jitterMs = jitterMs;
        }
    }
    static final class Limit {
        final int limit;
        final boolean blocking;
        Limit(int limit, boolean blocking) { this.limit = limit; this.blocking = blocking; }
    }
    static final class Transaction {
        final String adapter;
        Transaction(String adapter) { this.adapter = adapter; }
    }
    static final class OpenTransactions {
        private static final ThreadLocal<java.util.Map<String, String>> OPEN = ThreadLocal.withInitial(java.util.Map::of);
        private OpenTransactions() {}
        static String of(String adapter) { return OPEN.get().get(adapter); }
        static Object within(String adapter, String identifier, Call call) {
            java.util.Map<String, String> outer = OPEN.get();
            java.util.Map<String, String> joined = new java.util.HashMap<>(outer);
            joined.put(adapter, identifier);
            OPEN.set(java.util.Collections.unmodifiableMap(joined));
            try { return call.run(); } finally { OPEN.set(outer); }
        }
    }
    static boolean retryable(RuntimeException error) {
        return !(error instanceof Failure) || String.valueOf(error.getMessage()).endsWith("-adapter");
    }
    interface Operation { Object run(Execution execution); }
    static final class Configuration {
        final java.util.Map<String, CacheAdapter> caches;
        final java.util.Map<String, HttpAdapter> http;
        final java.util.Map<String, EntityAdapter> entities;
        final Authorization authorize;
        final Authorization authorizeEntity;
        Configuration(java.util.Map<String, CacheAdapter> caches, java.util.Map<String, HttpAdapter> http, java.util.Map<String, EntityAdapter> entities, Authorization authorize) {
            this(caches, http, entities, authorize, null);
        }
        Configuration(java.util.Map<String, CacheAdapter> caches, java.util.Map<String, HttpAdapter> http, java.util.Map<String, EntityAdapter> entities, Authorization authorize, Authorization authorizeEntity) {
            this.caches = java.util.Map.copyOf(caches); this.http = java.util.Map.copyOf(http); this.entities = java.util.Map.copyOf(entities); this.authorize = authorize; this.authorizeEntity = authorizeEntity;
        }
        Configuration() { this(java.util.Map.of(), java.util.Map.of(), java.util.Map.of(), null); }
    }
    static final class Entry {
        final boolean hit;
        final Object value;
        final long expiresAt;
        Entry() { hit = false; value = null; expiresAt = 0; }
        Entry(Object value, long expiresAt) { hit = true; this.value = value; this.expiresAt = expiresAt; }
    }
    static final class Policy {
        final String name;
        final int ttlMs, maxBytes, maxEntries;
        Policy(String name, int ttlMs, int maxBytes, int maxEntries) {
            if (name == null || name.isEmpty() || bytes(name) > 128 || ttlMs < 1 || ttlMs > 86400000 || maxBytes < 1 || maxBytes > 1048576 || maxEntries < 1 || maxEntries > 10000) throw fail("cache-policy");
            this.name = name; this.ttlMs = ttlMs; this.maxBytes = maxBytes; this.maxEntries = maxEntries;
        }
    }
    static final class Write {
        final Execution execution;
        final Policy policy;
        final long expiresAt;
        Write(Execution execution, Policy policy, long expiresAt) { this.execution = execution; this.policy = policy; this.expiresAt = expiresAt; }
    }
    static final class Exchange {
        final String client, method, path;
        final Object[][] query, headers, pathVariables;
        final int body;
        Exchange(String client, String method, String path) { this(client, method, path, new Object[0][], new Object[0][], -1); }
        Exchange(String client, String method, String path, Object[][] query, Object[][] headers, int body) {
            this(client, method, path, query, headers, body, new Object[0][]);
        }
        Exchange(String client, String method, String path, Object[][] query, Object[][] headers, int body, Object[][] pathVariables) {
            this.client = client; this.method = method; this.path = path;
            this.query = query; this.headers = headers; this.body = body; this.pathVariables = pathVariables;
        }
        boolean declared() { return query.length > 0 || headers.length > 0 || body >= 0 || pathVariables.length > 0; }
        Exchange filled(Object[] arguments) {
            if (pathVariables.length == 0) return this;
            String filled = path;
            for (Object[] pair : pathVariables) {
                int index = (int)pair[1];
                Object value = index >= 0 && index < arguments.length ? arguments[index] : null;
                String text = value instanceof String || value instanceof Number || value instanceof Boolean ? String.valueOf(value) : null;
                if (text == null || !text.matches("[A-Za-z0-9._~-]{1,256}") || text.equals(".") || text.equals("..")) throw fail("http-request");
                filled = filled.replace("{" + pair[0] + "}", text);
            }
            return new Exchange(client, method, filled, query, headers, body, pathVariables);
        }
    }
    static final class Entity {
        final String name, adapter, table, id;
        Entity(String name, String adapter, String table, String id) { this.name = name; this.adapter = adapter; this.table = table; this.id = id; }
    }
    static Context create(String name, Options options) {
        Execution execution = new Execution(options, null); execution.check();
        if (!configurationActive(name, options)) throw fail("inactive-configuration");
        String setupKey = setupKey(name, options);
        return new Context(options, acquireSetup(setupKey, name, execution), setupKey);
    }
    static final class SharedSetup {
        final Configuration configuration;
        int users = 1;
        SharedSetup(Configuration configuration) { this.configuration = configuration; }
    }
    private static final java.util.Map<String, SharedSetup> SETUPS = new java.util.HashMap<>();
    static String setupKey(String name, Options options) {
        return name + new java.util.TreeSet<>(options.profiles) + new java.util.TreeMap<>(options.properties);
    }
    static synchronized Configuration acquireSetup(String setupKey, String name, Execution execution) {
        SharedSetup shared = SETUPS.get(setupKey);
        if (shared != null) { shared.users++; return shared.configuration; }
        Configuration configuration;
        try { configuration = configure(name, execution); } catch (Failure error) { throw error; } catch (RuntimeException error) { throw fail("configuration"); }
        execution.check(); if (configuration == null) throw fail("configuration");
        SETUPS.put(setupKey, new SharedSetup(configuration));
        return configuration;
    }
    static synchronized void releaseSetup(String setupKey) {
        SharedSetup shared = SETUPS.get(setupKey);
        if (shared == null) return;
        shared.users--;
        if (shared.users == 0) SETUPS.remove(setupKey);
    }
    static Configuration configure(String name, Execution execution) {
        switch (name) { __CONFIGURATIONS__ default: throw fail("configuration"); }
    }
    static boolean configurationActive(String name, Options options) {
        switch (name) { __SELECTIONS__ default: return true; }
    }
    static boolean active(Options options, String[] profiles, String property, String expected, boolean matchIfMissing) {
        if (property != null) for (String key : options.properties.keySet()) if (key.startsWith(property + "_")) return false;
        boolean profile = profiles.length == 0;
        for (String name : profiles) if (options.profiles.contains(name)) profile = true;
        return profile && (property == null || (options.properties.containsKey(property) ? options.properties.get(property).equals(expected) : matchIfMissing));
    }
    static final class Property {
        final String field, name, type;
        final Object fallback;
        final boolean hasDefault;
        final Number minimum, maximum;
        Property(String field, String name, String type, Object fallback, boolean hasDefault, Number minimum, Number maximum) {
            this.field = field; this.name = name; this.type = type; this.fallback = fallback; this.hasDefault = hasDefault; this.minimum = minimum; this.maximum = maximum;
        }
    }
    static java.util.Map<String, String> propertyInputs(java.util.Map<?, ?> values) {
        if (values != null && (values.containsKey("tac") || values.containsKey("yon"))) {
            java.util.Map<String, String> normalized = new java.util.LinkedHashMap<>();
            for (java.util.Map.Entry<String, String> item : applicationInputs(values).entrySet())
                if (item.getKey().startsWith(ENVIRONMENT + ".")) normalized.put(item.getKey().replace('.', '_').toUpperCase(java.util.Locale.ROOT), item.getValue());
            values = normalized;
        }
        if (values == null || values.size() > 256) throw fail("configuration");
        java.util.Map<String, String> result = new java.util.LinkedHashMap<>();
        for (java.util.Map.Entry<?, ?> entry : values.entrySet()) {
            if (!(entry.getKey() instanceof String) || !(entry.getValue() instanceof String)) throw fail("configuration");
            String key = (String)entry.getKey(), value = (String)entry.getValue();
            if (key.length() > 256 || !key.matches("[A-Z][A-Z0-9_]*") || !key.startsWith(ENVIRONMENT.equals("yon") ? "YON_" : "TAC_") || bytes(value) > 4096) throw fail("configuration");
            result.put(key, value);
        }
        try { copy(result, 65536); } catch (RuntimeException error) { throw fail("configuration"); }
        return java.util.Collections.unmodifiableMap(result);
    }
    static java.util.Map<String, Object> properties(String name, java.util.Map<?, ?> values) {
        switch (name) { __PROPERTIES__ default: throw fail("configuration-properties"); }
    }
    static java.util.Map<String, Object> values(String name, java.util.Map<?, ?> application) {
        switch (name) { __VALUES__ default: throw fail("configuration-properties"); }
    }
    static java.util.Map<String, String> applicationInputs(java.util.Map<?, ?> application) {
        try {
            java.util.Map<String, Object> root = map(copy(application, 65536));
            java.util.Map<String, String> flat = new java.util.LinkedHashMap<>();
            if (!root.containsKey("tac") && !root.containsKey("yon")) throw fail("configuration-properties");
            for (String key : root.keySet()) {
                if (!key.equals("tac") && !key.equals("yon")) continue;
                flattenApplication(map(root.get(key)), key, 0, flat);
            }
            return flat;
        } catch (RuntimeException error) { throw fail("configuration-properties"); }
    }
    static void flattenApplication(Object value, String path, int depth, java.util.Map<String, String> flat) {
        if (depth > 16 || path.length() > 256 || flat.size() >= 256) throw fail("configuration-properties");
        if (path.equals("yon.server.port") && (!(value instanceof Number) || ((Number)value).doubleValue() != Math.rint(((Number)value).doubleValue()))) throw fail("configuration-properties");
        if (value instanceof java.util.Map) {
            if (depth > 0 && map(value).isEmpty()) flat.put(path + ".", "");
            for (java.util.Map.Entry<String, Object> item : map(value).entrySet()) {
                if (!item.getKey().matches("[a-z][a-z0-9]*")) throw fail("configuration-properties");
                flattenApplication(item.getValue(), path + "." + item.getKey(), depth + 1, flat);
            }
        } else {
            if (value == null || value instanceof java.util.List) { flat.put(path + ".", ""); return; }
            if (!(value instanceof String) && !(value instanceof Boolean) && !(value instanceof Number)) throw fail("configuration-properties");
            String text = value instanceof Number && ((Number)value).doubleValue() == Math.rint(((Number)value).doubleValue()) ? Long.toString(((Number)value).longValue()) : value.toString();
            if (bytes(text) > 4096 || flat.size() >= 256) throw fail("configuration-properties");
            flat.put(path, text);
        }
    }
    static java.util.Map<String, Object> bindValues(Property[] fields, java.util.Map<?, ?> application) {
        java.util.Map<String, String> values = applicationInputs(application);
        java.util.Map<String, Object> result = new java.util.LinkedHashMap<>();
        for (Property field : fields) {
            for (String path : values.keySet()) if (path.startsWith(field.name + ".")) throw fail("configuration-properties");
            Object value = propertyValue(field, values.get(field.name));
            if (value instanceof Number) {
                double numeric = ((Number)value).doubleValue();
                if (!Double.isFinite(numeric) || (field.minimum != null && numeric < field.minimum.doubleValue()) || (field.maximum != null && numeric > field.maximum.doubleValue())) throw fail("configuration-properties");
            }
            result.put(field.field, value);
        }
        try { return map(copy(result, 65536)); } catch (RuntimeException error) { throw fail("configuration-properties"); }
    }
    static java.util.Map<String, Object> bind(String prefix, Property[] fields, java.util.Map<?, ?> supplied) {
        java.util.Map<String, String> values;
        try { values = propertyInputs(supplied); } catch (RuntimeException error) { throw fail("configuration-properties"); }
        java.util.Set<String> known = new java.util.HashSet<>();
        for (Property field : fields) known.add(prefix + field.name);
        for (String key : values.keySet()) if (key.startsWith(prefix) && !known.contains(key)) throw fail("configuration-properties");
        java.util.Map<String, Object> result = new java.util.LinkedHashMap<>();
        for (Property field : fields) {
            Object value = propertyValue(field, values.get(prefix + field.name));
            if (value instanceof Number) {
                double numeric = ((Number)value).doubleValue();
                if (!Double.isFinite(numeric) || (field.minimum != null && numeric < field.minimum.doubleValue()) || (field.maximum != null && numeric > field.maximum.doubleValue())) throw fail("configuration-properties");
            }
            result.put(field.field, value);
        }
        try { return map(copy(result, 65536)); } catch (RuntimeException error) { throw fail("configuration-properties"); }
    }
    static Object propertyValue(Property field, String raw) {
        Object value;
        if (raw != null) value = parseProperty(raw, field.type);
        else { if (!field.hasDefault) throw fail("configuration-properties"); value = field.fallback; }
        if (field.type.equals("string") && (!(value instanceof String) || bytes((String)value) > 4096)) throw fail("configuration-properties");
        return value;
    }
    static Object parseProperty(String value, String type) {
        if (type.equals("string")) return value;
        if (type.equals("boolean")) { if (!value.equals("true") && !value.equals("false")) throw fail("configuration-properties"); return value.equals("true"); }
        try {
            if (type.equals("integer") && value.matches("-?(0|[1-9][0-9]*)")) {
                long result = Long.parseLong(value); if (result < -9007199254740991L || result > 9007199254740991L) throw fail("configuration-properties"); return result;
            }
            if (type.equals("number") && value.matches("-?(0|[1-9][0-9]*)(\\.[0-9]+)?([eE][+-]?[0-9]+)?")) {
                double result = Double.parseDouble(value); if (!Double.isFinite(result) || Math.abs(result) > 9007199254740991d) throw fail("configuration-properties"); return result;
            }
        } catch (RuntimeException error) { throw fail("configuration-properties"); }
        throw fail("configuration-properties");
    }
    static final class Rule {
        final String field;
        final boolean notNull, notBlank;
        final Long sizeMin, sizeMax;
        final Number minimum, maximum;
        Rule(String field, boolean notNull, boolean notBlank, Long sizeMin, Long sizeMax, Number minimum, Number maximum) {
            this.field = field; this.notNull = notNull; this.notBlank = notBlank; this.sizeMin = sizeMin; this.sizeMax = sizeMax; this.minimum = minimum; this.maximum = maximum;
        }
    }
    static java.util.Map<String, Object> validate(String name, Object value) {
        switch (name) { __VALIDATIONS__ default: throw new Failure("validation", "$.schema"); }
    }
    static java.util.Map<String, Object> validateRules(Object input, Rule[] rules) {
        Object copied;
        try { copied = copy(input, 1048576); } catch (RuntimeException error) { throw new Failure("validation", "$.json"); }
        if (!(copied instanceof java.util.Map<?, ?>)) throw new Failure("validation", "$.object");
        java.util.Map<String, Object> value = map(copied);
        for (Rule rule : rules) {
            Object field = value.get(rule.field);
            if (rule.notNull && field == null) throw new Failure("validation", rule.field + ".notNull");
            if (rule.notBlank && (!(field instanceof String) || ((String)field).codePoints().allMatch(Behaviors::whiteSpace))) throw new Failure("validation", rule.field + ".notBlank");
            if (field == null) continue;
            if (rule.sizeMin != null || rule.sizeMax != null) {
                long size;
                if (field instanceof String) size = bytes((String)field);
                else if (field instanceof java.util.List<?>) size = ((java.util.List<?>)field).size();
                else if (field instanceof java.util.Map<?, ?>) size = ((java.util.Map<?, ?>)field).size();
                else throw new Failure("validation", rule.field + ".size");
                if ((rule.sizeMin != null && size < rule.sizeMin) || (rule.sizeMax != null && size > rule.sizeMax)) throw new Failure("validation", rule.field + ".size");
            }
            if (rule.minimum != null && (!(field instanceof Number) || ((Number)field).doubleValue() < rule.minimum.doubleValue())) throw new Failure("validation", rule.field + ".min");
            if (rule.maximum != null && (!(field instanceof Number) || ((Number)field).doubleValue() > rule.maximum.doubleValue())) throw new Failure("validation", rule.field + ".max");
        }
        return value;
    }
    static boolean whiteSpace(int code) {
        return (code >= 9 && code <= 13) || code == 32 || code == 133 || code == 160 || code == 5760 || (code >= 8192 && code <= 8202) || code == 8232 || code == 8233 || code == 8239 || code == 8287 || code == 12288;
    }
    static Entity entity(String name) {
        switch (name) { __ENTITIES__ default: throw fail("entity"); }
    }
    static final class Context implements AutoCloseable {
        final Options options;
        final Configuration configuration;
        private volatile boolean closed;
        private int active;
        private final String setupKey;
        private final java.util.Set<String> revalidations = new java.util.HashSet<>();
        Context(Options options, Configuration configuration) { this(options, configuration, null); }
        Context(Options options, Configuration configuration, String setupKey) { this.options = options; this.configuration = configuration; this.setupKey = setupKey; }
        public void close() { dispose(); }
        void dispose() {
            synchronized (this) { if (closed) return; closed = true; }
            if (setupKey != null) releaseSetup(setupKey);
        }
        Object operation(Operation operation) {
            synchronized (this) { if (closed || active >= 64) throw fail("admission"); active++; }
            Execution execution = new Execution(options, this);
            try { execution.check(); Object value = operation.run(execution); execution.check(); return value; }
            finally { synchronized (this) { active--; } }
        }
        private CacheAdapter cacheAdapter(String name) {
            CacheAdapter configured = configuration.caches.get(name);
            return configured != null ? configured : BehaviorBuiltins.cache(name);
        }
        private EntityAdapter entityAdapter(String name) {
            EntityAdapter configured = configuration.entities.get(name);
            return configured != null ? configured : BehaviorBuiltins.entities(name);
        }
        private boolean builtinCache(String name) {
            return configuration.authorize == null && !configuration.caches.containsKey(name) && BehaviorBuiltins.cache(name) != null;
        }
        private boolean builtinEntities(String name) {
            return configuration.authorizeEntity == null && !configuration.entities.containsKey(name) && BehaviorBuiltins.entities(name) != null;
        }
        private final java.util.Map<String, java.util.concurrent.Semaphore> slots = new java.util.concurrent.ConcurrentHashMap<>();
        private Object transacted(Transaction policy, Execution execution, Call call) {
            if (policy == null || OpenTransactions.of(policy.adapter) != null) return call.run();
            EntityAdapter adapter = entityAdapter(policy.adapter);
            if (adapter == null) throw fail("entity-adapter");
            Object begun = control(adapter, execution, "begin", null);
            Object identifier = begun instanceof java.util.Map<?, ?> ? map(begun).get("transaction") : null;
            if (!(identifier instanceof String) || ((String)identifier).isEmpty() || bytes((String)identifier) > 128) throw fail("entity-adapter");
            Object value;
            try { value = OpenTransactions.within(policy.adapter, (String)identifier, call); }
            catch (RuntimeException error) {
                try { control(adapter, execution, "rollback", (String)identifier); } catch (RuntimeException ignored) { }
                throw error;
            }
            control(adapter, execution, "commit", (String)identifier);
            return value;
        }
        private Object control(EntityAdapter adapter, Execution execution, String action, String identifier) {
            java.util.Map<String, Object> command = identifier == null
                ? record("operation", "transaction", "action", action)
                : record("operation", "transaction", "action", action, "transaction", identifier);
            try { Object result = adapter.execute(command, execution); execution.check(); return result; }
            catch (Failure error) { throw error; } catch (RuntimeException error) { throw fail("entity-adapter"); }
        }
        Object resilient(String type, String method, Retry retry, Limit limit, Transaction transactional, Execution execution, Call call) {
            Call body = () -> transacted(transactional, execution, call);
            if (limit == null) return attempt(retry, execution, body);
            java.util.concurrent.Semaphore slot = slots.computeIfAbsent(type + "." + method, key -> new java.util.concurrent.Semaphore(limit.limit, true));
            boolean acquired;
            try {
                acquired = limit.blocking
                    ? slot.tryAcquire(Math.max(0L, execution.monotonicDeadline - System.nanoTime()), java.util.concurrent.TimeUnit.NANOSECONDS)
                    : slot.tryAcquire();
            } catch (InterruptedException error) { Thread.currentThread().interrupt(); throw fail("aborted"); }
            if (!acquired) throw limit.blocking ? fail("timeout") : fail("capacity");
            try { return attempt(retry, execution, body); } finally { slot.release(); }
        }
        private Object attempt(Retry retry, Execution execution, Call call) {
            if (retry == null) return call.run();
            for (int attempt = 0; ; attempt++) {
                try { return call.run(); }
                catch (RuntimeException error) {
                    if (attempt >= retry.maxRetries || !retryable(error)) throw error;
                    execution.check();
                    long wait = Math.round(Math.min(retry.delayMs * Math.pow(retry.multiplier, attempt), retry.maxDelayMs) + Math.random() * retry.jitterMs);
                    if (wait > 0) {
                        try { Thread.sleep(wait); } catch (InterruptedException interrupted) { Thread.currentThread().interrupt(); throw fail("aborted"); }
                    }
                    execution.check();
                }
            }
        }
        Object invoke(String type, String method, Object[] arguments, int[] indices, Policy policy, Exchange exchange, Call call) {
            return invoke(type, method, arguments, indices, policy, exchange, null, null, null, call);
        }
        Object mutate(String type, String target, Object[] arguments, int[] indices, Policy policy, boolean put, Exchange exchange, Call call) {
            return mutate(type, target, arguments, indices, policy, put, exchange, null, null, null, call);
        }
        Object invoke(String type, String method, Object[] arguments, int[] indices, Policy policy, Exchange declaredExchange, Retry retry, Limit limit, Transaction transactional, Call call) {
            Exchange exchange = declaredExchange == null ? null : declaredExchange.filled(arguments);
            return operation(execution -> {
                String key = null;
                CacheAdapter cache = null;
                if (policy != null) {
                    key = cacheKey(type, method, arguments, indices, policy, execution, "cache:read");
                    cache = cacheAdapter(policy.name);
                    if (cache == null) throw fail("cache-adapter");
                    Entry entry;
                    try { entry = cache.get(key, execution); } catch (RuntimeException error) { throw fail("cache-adapter"); }
                    execution.check(); if (entry == null) throw fail("cache-entry");
                    long now = System.currentTimeMillis();
                    if (entry.hit && entry.expiresAt > now + policy.ttlMs) throw fail("cache-entry");
                    if (entry.hit && entry.expiresAt > now) return copy(entry.value, policy.maxBytes);
                }
                boolean revalidate = key != null && revalidations.remove(key);
                Object value = exchange != null && exchange.declared()
                    ? exchangeRequest(exchange, arguments)
                    : resilient(type, method, retry, limit, transactional, execution, call);
                execution.check();
                if (exchange != null) value = send(value, exchange, execution, key != null && exchange.method.equals("GET"), revalidate);
                value = copy(value, policy == null ? 1048576 : policy.maxBytes);
                if (cache != null) {
                    try { cache.set(key, copy(value, policy.maxBytes), new Write(execution, policy, System.currentTimeMillis() + policy.ttlMs)); }
                    catch (RuntimeException error) { throw fail("cache-adapter"); }
                }
                execution.check(); return value;
            });
        }
        Object mutate(String type, String target, Object[] arguments, int[] indices, Policy policy, boolean put, Exchange declaredExchange, Retry retry, Limit limit, Transaction transactional, Call call) {
            Exchange exchange = declaredExchange == null ? null : declaredExchange.filled(arguments);
            return operation(execution -> {
                String key = cacheKey(type, target, arguments, indices, policy, execution, put ? "cache:put" : "cache:evict");
                CacheAdapter cache = cacheAdapter(policy.name);
                if (cache == null) throw fail("cache-adapter");
                Object value = resilient(type, target, retry, limit, transactional, execution, call); execution.check();
                if (exchange != null) value = send(value, exchange, execution, false, false);
                value = copy(value, put ? policy.maxBytes : 1048576);
                execution.check();
                try {
                    if (put) cache.set(key, value, new Write(execution, policy, System.currentTimeMillis() + policy.ttlMs));
                    else cache.delete(key, execution);
                } catch (RuntimeException error) { throw fail("cache-adapter"); }
                revalidations.add(key);
                execution.check(); return value;
            });
        }
        String cacheKey(String type, String method, Object[] arguments, int[] indices, Policy policy, Execution execution, String operation) {
            java.util.List<Object> selected = new java.util.ArrayList<>();
            for (int index : indices) { if (index < 0 || index >= arguments.length) throw fail("cache-key"); selected.add(copy(arguments[index], 4096)); }
            Object immutable = copy(selected, 4096);
            if (!builtinCache(policy.name)) {
                if (configuration.authorize == null) throw fail("authorization");
                java.util.Map<String, Object> call = record("operation", operation, "cache", policy.name, "type", type, "method", method, "keyArguments", immutable);
                boolean allowed;
                try { allowed = configuration.authorize.allow(call, execution); } catch (RuntimeException error) { throw fail("authorization"); }
                execution.check(); if (!allowed) throw fail("authorization");
            }
            Object key = java.util.Arrays.asList(options.partition, policy.name, MODULE, type, method, immutable);
            return encode(copy(key, 4096));
        }
        void evict(String type, String method, Object[] arguments) {
            operation(execution -> { __EVICTIONS__ throw fail("cache-method"); });
        }
        Repository repository(String name) { return new Repository(this, entity(name)); }
        Object query(Entity entity, String kind, String[] fields, Object[][] where, String[][] orderBy, Object input, int defaultSize, int maxSize) {
            return operation(execution -> {
                java.util.Map<String, Object> query = queryDescriptor(kind, fields, where, orderBy, input, defaultSize, maxSize);
                String transaction = OpenTransactions.of(entity.adapter);
                java.util.Map<String, Object> command = map(copy(transaction == null
                    ? record("operation", "query", "entity", entity.name, "table", entity.table, "idField", entity.id, "query", query)
                    : record("operation", "query", "entity", entity.name, "table", entity.table, "idField", entity.id, "query", query, "transaction", transaction), 1048576));
                if (!builtinEntities(entity.adapter)) {
                    Authorization authorize = configuration.authorizeEntity;
                    if (authorize == null) throw fail("authorization");
                    boolean allowed;
                    try { allowed = authorize.allow(command, execution); } catch (RuntimeException error) { throw fail("authorization"); }
                    execution.check(); if (!allowed) throw fail("authorization");
                }
                EntityAdapter adapter = entityAdapter(entity.adapter); if (adapter == null) throw fail("entity-adapter");
                Object result;
                try { result = adapter.execute(command, execution); } catch (RuntimeException error) { throw fail("entity-adapter"); }
                execution.check(); result = copy(result, 1048576);
                if (kind.equals("count") || kind.equals("delete")) return integer(result, 0, 9007199254740991L, "query-result");
                if (kind.equals("exists")) { if (!(result instanceof Boolean)) throw fail("query-result"); return result; }
                if (!(result instanceof java.util.List<?>)) throw fail("query-result");
                java.util.List<?> rows = (java.util.List<?>)result;
                if (rows.size() > (Long)map(query.get("page")).get("size")) throw fail("query-result");
                for (Object row : rows) if (!(row instanceof java.util.Map<?, ?>)) throw fail("query-result");
                execution.check();
                if (kind.equals("findOne")) return rows.isEmpty() ? null : rows.get(0);
                return result;
            });
        }
        RestClient restClient(String name) {
            if (name == null || name.isEmpty() || name.length() > 128) throw fail("http-adapter");
            return new RestClient(this, name);
        }
        static Object exchangeRequest(Exchange metadata, Object[] arguments) {
            java.util.Map<String, Object> query = new java.util.LinkedHashMap<>();
            java.util.Map<String, Object> headers = new java.util.LinkedHashMap<>();
            for (Object[] pair : metadata.query) query.put((String)pair[0], argument(arguments, (int)pair[1]));
            for (Object[] pair : metadata.headers) headers.put((String)pair[0], argument(arguments, (int)pair[1]));
            java.util.Map<String, Object> descriptor = new java.util.LinkedHashMap<>();
            descriptor.put("query", query); descriptor.put("headers", headers);
            if (metadata.body >= 0) descriptor.put("body", argument(arguments, metadata.body));
            return descriptor;
        }
        private static Object argument(Object[] arguments, int index) {
            if (index < 0 || index >= arguments.length) throw fail("http-request");
            return arguments[index];
        }
        Object exchange(Object descriptor, Exchange metadata, Execution execution) {
            HttpAdapter adapter = configuration.http.get(metadata.client); if (adapter == null) throw fail("http-adapter");
            return adapted(adapter, request(descriptor, metadata), execution);
        }
        Object send(Object descriptor, Exchange metadata, Execution execution, boolean publicRead, boolean revalidate) {
            java.util.Map<String, Object> request = request(descriptor, metadata);
            HttpAdapter adapter = configuration.http.get(metadata.client);
            if (adapter != null) return adapted(adapter, request, execution);
            java.util.Map<String, Object> browser = new java.util.LinkedHashMap<>(request);
            browser.put("deadline", execution.deadline); browser.put("publicRead", publicRead); browser.put("revalidate", revalidate);
            execution.check();
            Object value = BehaviorBuiltins.exchange(java.util.Collections.unmodifiableMap(browser), execution);
            execution.check(); return copy(value, 1048576);
        }
        static java.util.Map<String, Object> request(Object descriptor, Exchange metadata) {
            java.util.Map<String, Object> request = new java.util.LinkedHashMap<>(map(copy(descriptor, 1048576)));
            for (String key : request.keySet()) if (!java.util.Set.of("query", "headers", "body").contains(key)) throw fail("http-request");
            validateRequest(request);
            if ((metadata.method.equals("GET") || metadata.method.equals("HEAD")) && request.containsKey("body")) throw fail("http-request");
            request.put("method", metadata.method); request.put("path", metadata.path);
            request.put("redirect", "error"); request.put("credentials", "omit"); request.put("maxResponseBytes", 1048576);
            return java.util.Collections.unmodifiableMap(request);
        }
        static Object adapted(HttpAdapter adapter, java.util.Map<String, Object> request, Execution execution) {
            try { Object value = adapter.exchange(request, execution); execution.check(); return copy(value, 1048576); }
            catch (Failure error) { throw error; } catch (RuntimeException error) { throw fail("http-adapter"); }
        }
    }
    static final class RestClient {
        private final Context context;
        private final String name;
        RestClient(Context context, String name) { this.context = context; this.name = name; }
        RestCall get(String path) { return new RestCall(context, name, "GET", path); }
        RestCall post(String path) { return new RestCall(context, name, "POST", path); }
        RestCall put(String path) { return new RestCall(context, name, "PUT", path); }
        RestCall patch(String path) { return new RestCall(context, name, "PATCH", path); }
        RestCall delete(String path) { return new RestCall(context, name, "DELETE", path); }
        RestCall head(String path) { return new RestCall(context, name, "HEAD", path); }
        RestCall options(String path) { return new RestCall(context, name, "OPTIONS", path); }
    }
    static final class RestCall {
        private final Context context;
        private final String name, method, path;
        private final java.util.Map<String, Object> query = new java.util.LinkedHashMap<>();
        private final java.util.Map<String, Object> headers = new java.util.LinkedHashMap<>();
        private Object body;
        private boolean hasBody;
        RestCall(Context context, String name, String method, String path) {
            if (!safePath(path)) throw fail("http-request");
            this.context = context; this.name = name; this.method = method; this.path = path;
        }
        RestCall query(java.util.Map<String, Object> values) { query.putAll(values); return this; }
        RestCall header(String key, String value) { headers.put(key, value); return this; }
        RestCall body(Object value) {
            if (method.equals("GET") || method.equals("HEAD")) throw fail("http-request");
            body = value; hasBody = true; return this;
        }
        Object retrieve() {
            return context.operation(execution -> {
                java.util.Map<String, Object> descriptor = new java.util.LinkedHashMap<>();
                descriptor.put("query", query); descriptor.put("headers", headers);
                if (hasBody) descriptor.put("body", body);
                return context.exchange(descriptor, new Exchange(name, method, path), execution);
            });
        }
    }
    static boolean safePath(String path) {
        if (path == null || !path.startsWith("/") || path.startsWith("//") || path.length() > 2048) return false;
        for (int index = 0; index < path.length(); index++) {
            char character = path.charAt(index);
            if (character <= ' ' || character == '\\' || character == '?' || character == '#' || character == '%') return false;
        }
        for (String part : path.split("/", -1)) if (part.equals(".") || part.equals("..")) return false;
        return true;
    }
    static final class Repository {
        final Context context;
        final Entity entity;
        Repository(Context context, Entity entity) { this.context = context; this.entity = entity; }
        Object find(java.util.Map<String, Object> input) { return execute("find", "page", page(input, 20, 100, new String[]{entity.id})); }
        Object save(Object value) {
            java.util.Map<String, Object> data = map(copy(value, 1048576));
            identifier(data.get(entity.id)); return execute("save", "value", data);
        }
        Object delete(Object id) { identifier(id); return execute("delete", "id", id); }
        Object execute(String operation, String key, Object value) {
            return context.operation(execution -> {
                EntityAdapter adapter = context.entityAdapter(entity.adapter); if (adapter == null) throw fail("entity-adapter");
                String transaction = OpenTransactions.of(entity.adapter);
                java.util.Map<String, Object> command = transaction == null
                    ? record("operation", operation, "entity", entity.name, "table", entity.table, "idField", entity.id, key, value)
                    : record("operation", operation, "entity", entity.name, "table", entity.table, "idField", entity.id, key, value, "transaction", transaction);
                try { Object result = adapter.execute(command, execution); execution.check(); return copy(result, 1048576); }
                catch (Failure error) { throw error; } catch (RuntimeException error) { throw fail("entity-adapter"); }
            });
        }
    }
    static void identifier(Object value) {
        if (value instanceof String) { if (((String)value).isEmpty() || bytes((String)value) > 256) throw fail("entity-id"); return; }
        integer(value, -9007199254740991L, 9007199254740991L, "entity-id");
    }
    static java.util.Map<String, Object> queryDescriptor(String kind, String[] fields, Object[][] where, String[][] orderBy, Object input, int defaultSize, int maxSize) {
        if (!java.util.Set.of("find", "findOne", "count", "exists", "delete").contains(kind)) throw fail("query");
        java.util.List<String> allowed = java.util.Arrays.asList(fields);
        java.util.List<Object> filters = new java.util.ArrayList<>();
        for (Object[] predicate : where) {
            if (predicate.length < 2 || predicate.length > 3 || !allowed.contains(predicate[0]) || !(predicate[1] instanceof String)) throw fail("query");
            String operator = (String)predicate[1];
            java.util.Map<String, Object> filter;
            if (operator.equals("isNull") || operator.equals("isNotNull")) {
                if (predicate.length != 2) throw fail("query");
                filter = record("field", predicate[0], "operator", operator);
            } else {
                if (predicate.length != 3) throw fail("query");
                Object value = copy(predicate[2], 65536);
                if (operator.equals("in")) {
                    if (!(value instanceof java.util.List<?>) || ((java.util.List<?>)value).size() > 100) throw fail("query-binding");
                    for (Object item : (java.util.List<?>)value) queryScalar(item);
                } else {
                    if (!java.util.Set.of("eq", "ne", "lt", "lte", "gt", "gte").contains(operator)) throw fail("query");
                    queryScalar(value);
                }
                filter = record("field", predicate[0], "operator", operator, "value", value);
            }
            filters.add(filter);
        }
        Object bindings = copy(filters, 65536);
        java.util.List<Object> ordering = new java.util.ArrayList<>();
        java.util.Set<String> seen = new java.util.HashSet<>();
        for (String[] order : orderBy) {
            if (order.length != 2 || !allowed.contains(order[0]) || !seen.add(order[0]) || !java.util.Set.of("asc", "desc").contains(order[1])) throw fail("query");
            ordering.add(record("field", order[0], "direction", order[1]));
        }
        if (kind.equals("findOne")) return map(copy(record("kind", "find", "where", bindings, "orderBy", ordering, "page", page(java.util.Map.of("size", 1L), 1, 1, fields)), 1048576));
        if (!kind.equals("find")) return map(copy(record("kind", kind, "where", bindings, "orderBy", ordering), 1048576));
        java.util.Map<String, Object> page = page(input, defaultSize, maxSize, fields);
        if (!((java.util.List<?>)page.get("sort")).isEmpty()) throw fail("pagination");
        return map(copy(record("kind", kind, "where", bindings, "orderBy", ordering, "page", page), 1048576));
    }
    static void queryScalar(Object value) {
        if (value != null && !(value instanceof String) && !(value instanceof Number) && !(value instanceof Boolean)) throw fail("query-binding");
    }
    static java.util.Map<String, Object> page(Object input, int defaultSize, int maxSize, String[] allowed) {
        java.util.Map<String, Object> value = input == null ? java.util.Map.of() : map(copy(input, 65536));
        for (String key : value.keySet()) if (!java.util.Set.of("page", "size", "sort", "offset").contains(key)) throw fail("pagination");
        long page = integer(value.getOrDefault("page", 0), 0, 1000000, "pagination");
        long size = integer(value.getOrDefault("size", defaultSize), 1, maxSize, "pagination");
        long offset = page * size; if (offset > 1000000) throw fail("pagination");
        Object sorts = value.getOrDefault("sort", java.util.List.of()); if (!(sorts instanceof java.util.List<?>)) throw fail("pagination");
        java.util.List<String> normalized = new java.util.ArrayList<>(); java.util.Set<String> seen = new java.util.HashSet<>();
        for (Object item : (java.util.List<?>)sorts) {
            if (!(item instanceof String)) throw fail("pagination"); String[] pair = ((String)item).split(":", -1);
            if (pair.length > 2 || !java.util.Arrays.asList(allowed).contains(pair[0]) || !seen.add(pair[0])) throw fail("pagination");
            String direction = pair.length == 1 ? "asc" : pair[1]; if (!direction.equals("asc") && !direction.equals("desc")) throw fail("pagination");
            normalized.add(pair[0] + ":" + direction);
        }
        return record("page", page, "size", size, "offset", offset, "sort", java.util.Collections.unmodifiableList(normalized));
    }
    static void validateRequest(java.util.Map<String, Object> request) {
        for (String key : java.util.List.of("query", "headers")) {
            if (!request.containsKey(key)) continue;
            java.util.Map<String, Object> fields = map(request.get(key));
            if (fields.size() > 128) throw fail("http-request");
            for (java.util.Map.Entry<String, Object> field : fields.entrySet()) {
                String name = field.getKey(); Object value = field.getValue();
                if (name.isEmpty() || bytes(name) > 256) throw fail("http-request");
                if (key.equals("query")) { if (!(value instanceof String || value instanceof Number || value instanceof Boolean)) throw fail("http-request"); continue; }
                if (!(value instanceof String) || bytes((String)value) > 8192 || !name.matches("[!#$%&'*+.^_`|~0-9A-Za-z-]{1,128}") || ((String)value).contains("\r") || ((String)value).contains("\n") || ((String)value).indexOf(0) >= 0) throw fail("http-request");
                if (java.util.Set.of("host", "connection", "content-length", "transfer-encoding", "cookie").contains(name.toLowerCase(java.util.Locale.ROOT))) throw fail("http-request");
            }
        }
    }
    static long integer(Object value, long minimum, long maximum, String code) {
        if (!(value instanceof Byte || value instanceof Short || value instanceof Integer || value instanceof Long)) throw fail(code);
        long number = ((Number)value).longValue(); if (number < minimum || number > maximum) throw fail(code); return number;
    }
    static int bytes(String value) { return value.getBytes(java.nio.charset.StandardCharsets.UTF_8).length; }
    static java.util.Map<String, Object> record(Object... pairs) {
        java.util.Map<String, Object> result = new java.util.LinkedHashMap<>();
        for (int i = 0; i < pairs.length; i += 2) result.put((String)pairs[i], pairs[i + 1]);
        return java.util.Collections.unmodifiableMap(result);
    }
    static java.util.Map<String, Object> map(Object value) {
        if (!(value instanceof java.util.Map<?, ?>)) throw fail("json-value");
        java.util.Map<String, Object> result = new java.util.LinkedHashMap<>();
        for (java.util.Map.Entry<?, ?> entry : ((java.util.Map<?, ?>)value).entrySet()) {
            if (!(entry.getKey() instanceof String)) throw fail("json-value"); result.put((String)entry.getKey(), entry.getValue());
        }
        return java.util.Collections.unmodifiableMap(result);
    }
    static final java.util.Set<String> GATEWAY_FAILURES = java.util.Set.of("invalid", "timeout", "capacity", "adapter");
    static Object gatewayResult(java.util.Map<?, ?> answer) {
        if (answer.containsKey("result")) return answer.get("result");
        Object kind = answer.get("error");
        if (kind instanceof String && GATEWAY_FAILURES.contains(kind)) throw gatewayFailure((String)kind);
        throw gatewayFailure("adapter");
    }
    static Failure gatewayFailure(String kind) { return new Failure(kind, null, "Gateway call failed."); }
    static Object mapValue(Object value, Object[][] plan) {
        java.util.Map<String, Object> source = map(copy(value, 1048576));
        java.util.Map<String, Object> output = new java.util.LinkedHashMap<>();
        for (Object[] entry : plan) {
            if (entry[1] == null) { output.put((String)entry[0], entry[2]); continue; }
            Object current = source;
            for (String name : (String[])entry[1]) {
                current = current instanceof java.util.Map<?, ?> ? ((java.util.Map<?, ?>)current).get(name) : null;
            }
            if (entry.length > 3 && entry[3] != null) current = mapShape(current, (Object[][])entry[3]);
            output.put((String)entry[0], current);
        }
        return copy(output, 1048576);
    }
    static Object mapShape(Object value, Object[][] plan) {
        if (value instanceof java.util.List<?> items) {
            java.util.List<Object> output = new java.util.ArrayList<>();
            for (Object item : items) output.add(mapShape(item, plan));
            return output;
        }
        return value instanceof java.util.Map<?, ?> ? mapValue(value, plan) : value;
    }
    static Object copy(Object value, int limit) {
        if (limit < 1 || limit > 1048576) throw fail("json-size");
        int[] budget = {0, 0}; Object result = copyValue(value, new java.util.IdentityHashMap<>(), 0, budget, limit);
        if (bytes(encode(result)) > limit) throw fail("json-size"); return result;
    }
    static Object copyValue(Object value, java.util.IdentityHashMap<Object, Boolean> seen, int depth, int[] budget, int limit) {
        budget[1] += 1;
        if (depth > 32 || ++budget[0] > 65536 || budget[1] > limit) throw fail("json-size");
        if (value == null || value instanceof Boolean) return value;
        if (value instanceof String) { budget[1] += bytes((String)value); if (budget[1] > limit) throw fail("json-size"); return value; }
        if (value instanceof Byte || value instanceof Short || value instanceof Integer || value instanceof Long) { integer(value, -9007199254740991L, 9007199254740991L, "json-number"); return value; }
        if (value instanceof Double || value instanceof Float) { if (!Double.isFinite(((Number)value).doubleValue())) throw fail("json-number"); return value; }
        if (seen.put(value, true) != null) throw fail("json-cycle");
        try {
            if (value instanceof java.util.List<?>) {
                if (((java.util.List<?>)value).size() > 65536) throw fail("json-size");
                java.util.List<Object> result = new java.util.ArrayList<>(); for (Object item : (java.util.List<?>)value) result.add(copyValue(item, seen, depth + 1, budget, limit));
                return java.util.Collections.unmodifiableList(result);
            }
            if (value instanceof java.util.Map<?, ?>) {
                if (((java.util.Map<?, ?>)value).size() > 65536) throw fail("json-size");
                java.util.Map<String, Object> result = new java.util.TreeMap<>();
                for (java.util.Map.Entry<?, ?> entry : ((java.util.Map<?, ?>)value).entrySet()) {
                    if (!(entry.getKey() instanceof String)) throw fail("json-value");
                    budget[1] += bytes((String)entry.getKey()); if (budget[1] > limit) throw fail("json-size");
                    result.put((String)entry.getKey(), copyValue(entry.getValue(), seen, depth + 1, budget, limit));
                }
                return java.util.Collections.unmodifiableMap(result);
            }
            throw fail("json-value");
        } finally { seen.remove(value); }
    }
    static String encode(Object value) {
        if (value == null) return "null";
        if (value instanceof String) return quote((String)value);
        if (value instanceof Number || value instanceof Boolean) return value.toString();
        java.util.StringJoiner joiner;
        if (value instanceof java.util.List<?>) { joiner = new java.util.StringJoiner(",", "[", "]"); for (Object item : (java.util.List<?>)value) joiner.add(encode(item)); return joiner.toString(); }
        joiner = new java.util.StringJoiner(",", "{", "}");
        for (java.util.Map.Entry<String, Object> entry : map(value).entrySet()) joiner.add(quote(entry.getKey()) + ":" + encode(entry.getValue()));
        return joiner.toString();
    }
    static String quote(String value) {
        StringBuilder output = new StringBuilder("\"");
        for (int i = 0; i < value.length(); i++) {
            char ch = value.charAt(i);
            if (Character.isHighSurrogate(ch)) { if (i + 1 >= value.length() || !Character.isLowSurrogate(value.charAt(i + 1))) throw fail("json-string"); output.append(ch).append(value.charAt(++i)); }
            else if (Character.isLowSurrogate(ch)) throw fail("json-string");
            else if (ch == '"' || ch == '\\') output.append('\\').append(ch);
            else if (ch < 32) output.append(String.format(java.util.Locale.ROOT, "\\u%04x", (int)ch));
            else output.append(ch);
        }
        return output.append('"').toString();
    }
}

final class BehaviorCalls {
    private static final java.util.LinkedHashSet<String> KEYS = new java.util.LinkedHashSet<>();

    private BehaviorCalls() {}

    static synchronized void record(String key) { KEYS.add(key); }

    static synchronized java.util.List<String> take() {
        java.util.List<String> keys = new java.util.ArrayList<>(KEYS);
        KEYS.clear();
        return keys;
    }
}
