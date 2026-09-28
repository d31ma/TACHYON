class RuntimeAdapters {
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
        if (!command.get("operation").equals("query")) {
            return command;
        }
        String kind = (String)Behaviors.map(command.get("query")).get("kind");
        if (kind.equals("count")) {
            return 1;
        }
        if (kind.equals("exists")) {
            return true;
        }
        return java.util.List.of(java.util.Map.of("id", "one", "query", command.get("query")));
    }
    static boolean authorizeCache(java.util.Map<String, Object> call, Behaviors.Execution current) {
        current.check();
        operation = (String)call.get("operation");
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
class RuntimeChecks {
    static int mutationCalls = 0;
    static void check(boolean value) {
        if (!value) {
            throw new AssertionError("behavior contract failed");
        }
    }
    static void fails(String code, Runnable call) {
        try {
            call.run();
            throw new AssertionError("expected " + code);
        } catch (Behaviors.Failure failure) {
            check(failure.getMessage().equals(code));
        }
    }
    static Behaviors.Policy catalogPolicy(int maxBytes) {
        return new Behaviors.Policy("catalog", 30000, maxBytes, 128);
    }
    static Object cached(Behaviors.Context context, Object argument) {
        return context.invoke("Catalog", "list", new Object[]{argument}, new int[]{0}, catalogPolicy(65536), null, () -> {
            RuntimeAdapters.calls++;
            return java.util.Map.of("id", RuntimeAdapters.calls);
        });
    }
    static Object mutate(Behaviors.Context context, boolean put, Behaviors.Call call) {
        return context.mutate("Catalog", "list", new Object[]{"mutation"}, new int[]{0}, catalogPolicy(64), put, null, call);
    }
    static Object countMutationCall() {
        mutationCalls++;
        return null;
    }
    static Object busyWaitFiveMilliseconds() {
        long until = System.nanoTime() + 5000000;
        while (System.nanoTime() < until) {
            Thread.onSpinWait();
        }
        return null;
    }
    static void mutationChecks() {
        Behaviors.Context context = Behaviors.create("RuntimeAdapters", new Behaviors.Options("mutation", new Behaviors.Signal()));
        for (boolean put : new boolean[]{false, true}) {
            checkMutation(context, put);
        }
        Behaviors.Exchange exchange = new Behaviors.Exchange("api", "POST", "/products");
        Object exchanged = context.mutate("Catalog", "list", new Object[]{"mutation"}, new int[]{0}, catalogPolicy(65536), true, exchange, () -> java.util.Map.of("body", java.util.Map.of("id", "one")));
        check(cached(context, "mutation").equals(exchanged));
        context.close();
        RuntimeAdapters.values.clear();
    }
    static void checkMutation(Behaviors.Context context, boolean put) {
        RuntimeAdapters.values.clear();
        Object previous = cached(context, "mutation");
        mutationCalls = 0;
        checkSuccessfulMutation(context, put, previous);
        checkDeniedMutation(context, put);
        checkRejectedMutationResults(context, put);
        checkInterruptedMutations(put);
        checkUnavailableCacheAdapters(context, put);
        checkOversizedMutationKey(context, put);
    }
    static void checkSuccessfulMutation(Behaviors.Context context, boolean put, Object previous) {
        int reads = RuntimeAdapters.reads;
        int writes = RuntimeAdapters.writes;
        int deletes = RuntimeAdapters.deletes;
        Object fresh = mutate(context, put, () -> {
            mutationCalls++;
            String expectedOperation = put ? "cache:put" : "cache:evict";
            check(RuntimeAdapters.operation.equals(expectedOperation));
            return java.util.Map.of("updated", true);
        });
        check(mutationCalls == 1 && RuntimeAdapters.reads == reads);
        if (put) {
            check(RuntimeAdapters.writes == writes + 1 && RuntimeAdapters.deletes == deletes);
            check(cached(context, "mutation").equals(fresh));
        } else {
            check(RuntimeAdapters.writes == writes && RuntimeAdapters.deletes == deletes + 1);
            check(!cached(context, "mutation").equals(previous));
        }
    }
    static void checkDeniedMutation(Behaviors.Context context, boolean put) {
        RuntimeAdapters.allow = false;
        fails("authorization", () -> mutate(context, put, () -> countMutationCall()));
        RuntimeAdapters.allow = true;
        check(mutationCalls == 1);
    }
    static void checkRejectedMutationResults(Behaviors.Context context, boolean put) {
        int writes = RuntimeAdapters.writes;
        int deletes = RuntimeAdapters.deletes;
        try {
            mutate(context, put, () -> {
                throw new IllegalArgumentException("domain");
            });
            throw new AssertionError("missing domain error");
        } catch (IllegalArgumentException expected) {
            check(expected.getMessage().equals("domain"));
        }
        fails("json-value", () -> mutate(context, put, Object::new));
        fails("json-size", () -> mutate(context, put, () -> "x".repeat(put ? 100 : 1048577)));
        fails("http-request", () -> context.mutate("Catalog", "list", new Object[]{"mutation"}, new int[]{0}, catalogPolicy(65536), put, new Behaviors.Exchange("api", "GET", "/products"), () -> java.util.Map.of("body", "invalid")));
        check(RuntimeAdapters.writes == writes && RuntimeAdapters.deletes == deletes);
    }
    static void checkInterruptedMutations(boolean put) {
        int writes = RuntimeAdapters.writes;
        int deletes = RuntimeAdapters.deletes;
        Behaviors.Signal signal = new Behaviors.Signal();
        Behaviors.Context aborted = Behaviors.create("RuntimeAdapters", new Behaviors.Options("mutation-abort", signal));
        fails("aborted", () -> mutate(aborted, put, () -> {
            signal.abort();
            return null;
        }));
        Behaviors.Context timed = Behaviors.create("RuntimeAdapters", new Behaviors.Options("mutation-timeout", new Behaviors.Signal(), 1));
        fails("timeout", () -> mutate(timed, put, () -> busyWaitFiveMilliseconds()));
        check(RuntimeAdapters.writes == writes && RuntimeAdapters.deletes == deletes);
    }
    static void checkUnavailableCacheAdapters(Behaviors.Context context, boolean put) {
        RuntimeAdapters.cacheUnavailable = true;
        fails("cache-adapter", () -> mutate(context, put, () -> countMutationCall()));
        RuntimeAdapters.cacheUnavailable = false;
        check(mutationCalls == 2);
        Behaviors.Configuration withoutCaches = new Behaviors.Configuration(java.util.Map.of(), java.util.Map.of(), java.util.Map.of(), (request, execution) -> true);
        Behaviors.Context missing = new Behaviors.Context(new Behaviors.Options("missing", new Behaviors.Signal()), withoutCaches);
        fails("cache-adapter", () -> mutate(missing, put, () -> countMutationCall()));
        check(mutationCalls == 2);
    }
    static void checkOversizedMutationKey(Behaviors.Context context, boolean put) {
        fails("json-size", () -> context.mutate("Catalog", "list", new Object[]{"x".repeat(5000)}, new int[]{0}, catalogPolicy(64), put, null, () -> countMutationCall()));
        check(mutationCalls == 2);
    }
    public static void main(String[] arguments) {
        Behaviors.Signal signal = new Behaviors.Signal();
        Behaviors.Context context = Behaviors.create("RuntimeAdapters", new Behaviors.Options("tenant-a", signal));
        checkCachedCalls(context);
        Behaviors.Context other = Behaviors.create("RuntimeAdapters", new Behaviors.Options("tenant-b", new Behaviors.Signal()));
        checkTenantPartition(other);
        checkCacheFailures(context);
        checkPagination();
        checkHttpExchange(context);
        checkRepository(context);
        checkJsonCopies();
        checkTimeout();
        checkCancellation(signal, context, other);
        mutationChecks();
        QueryChecks.run();
        ConfigurationChecks.run();
        ValidationChecks.run();
        System.out.println("Java runtime adapter and boundary checks passed");
    }
    static void checkCachedCalls(Behaviors.Context context) {
        Object first = cached(context, "one");
        check(first.equals(cached(context, "one")));
        check(RuntimeAdapters.calls == 1 && RuntimeAdapters.authorized == 2);
        RuntimeAdapters.allow = false;
        fails("authorization", () -> cached(context, "one"));
        fails("authorization", () -> context.evict("Catalog", "list", new Object[]{"one"}));
        RuntimeAdapters.allow = true;
        context.evict("Catalog", "list", new Object[]{"one"});
        cached(context, "one");
        check(RuntimeAdapters.calls == 2);
    }
    static void checkTenantPartition(Behaviors.Context other) {
        cached(other, "one");
        check(RuntimeAdapters.calls == 3);
        for (String key : RuntimeAdapters.values.keySet()) {
            check(key.contains(Behaviors.MODULE));
        }
    }
    static void checkCacheFailures(Behaviors.Context context) {
        RuntimeAdapters.cacheUnavailable = true;
        int callsBeforeOutage = RuntimeAdapters.calls;
        fails("cache-adapter", () -> cached(context, "one"));
        check(RuntimeAdapters.calls == callsBeforeOutage);
        RuntimeAdapters.cacheUnavailable = false;
        RuntimeAdapters.values.replaceAll((key, entry) -> new Behaviors.Entry(entry.value, Long.MAX_VALUE));
        fails("cache-entry", () -> cached(context, "one"));
        RuntimeAdapters.values.clear();
    }
    static void checkPagination() {
        java.util.Map<String, Object> page = Behaviors.page(java.util.Map.of("page", 2, "size", 5, "sort", java.util.List.of("id")), 20, 100, new String[]{"id"});
        check(page.get("offset").equals(10L) && page.get("sort").equals(java.util.List.of("id:asc")));
        fails("pagination", () -> Behaviors.page(java.util.Map.of("sort", java.util.List.of("id", "id")), 20, 100, new String[]{"id"}));
    }
    static Object loadProducts(Behaviors.Context context, Behaviors.Call call) {
        return context.invoke("Client", "load", new Object[]{}, new int[]{}, null, new Behaviors.Exchange("api", "GET", "/products"), call);
    }
    static void checkHttpExchange(Behaviors.Context context) {
        Object response = loadProducts(context, () -> java.util.Map.of("query", java.util.Map.of("q", "name")));
        check(Behaviors.map(response).get("path").equals("/products"));
        fails("http-request", () -> loadProducts(context, () -> java.util.Map.of("headers", java.util.Map.of("X-Test", "bad\r\nvalue"))));
        fails("http-request", () -> loadProducts(context, () -> java.util.Map.of("headers", java.util.Map.of("Host", "attacker.example"))));
        fails("http-request", () -> loadProducts(context, () -> java.util.Map.of("body", "forbidden")));
    }
    static void checkRepository(Behaviors.Context context) {
        Behaviors.Repository repository = context.repository("Product");
        check(Behaviors.map(repository.find(java.util.Map.of())).get("table").equals("products"));
        check(Behaviors.map(repository.save(java.util.Map.of("id", "one"))).get("operation").equals("save"));
        check(Behaviors.map(repository.delete("one")).get("operation").equals("delete"));
        fails("entity-id", () -> repository.save(java.util.Map.of("id", "")));
    }
    static void checkJsonCopies() {
        fails("json-number", () -> Behaviors.copy(Double.NaN, 100));
        fails("json-number", () -> Behaviors.copy(9007199254740992L, 100));
        fails("json-value", () -> Behaviors.copy(new Object(), 100));
        fails("json-size", () -> Behaviors.copy("x".repeat(1000), 100));
        java.util.List<Object> cyclic = new java.util.ArrayList<>();
        cyclic.add(cyclic);
        fails("json-cycle", () -> Behaviors.copy(cyclic, 100));
        java.util.List<Object> source = new java.util.ArrayList<>();
        source.add("old");
        Object immutable = Behaviors.copy(source, 100);
        source.set(0, "new");
        check(immutable.equals(java.util.List.of("old")));
    }
    static void checkTimeout() {
        Behaviors.Context timed = Behaviors.create("RuntimeAdapters", new Behaviors.Options("timeout", new Behaviors.Signal(), 1));
        fails("timeout", () -> timed.invoke("Slow", "run", new Object[]{}, new int[]{}, null, null, () -> busyWaitFiveMilliseconds()));
    }
    static void checkCancellation(Behaviors.Signal signal, Behaviors.Context context, Behaviors.Context other) {
        signal.abort();
        fails("aborted", () -> cached(context, "one"));
        fails("aborted", () -> other.operation(execution -> {
            other.dispose();
            check(execution.signal.aborted());
            return null;
        }));
        fails("admission", () -> cached(other, "one"));
    }
}
