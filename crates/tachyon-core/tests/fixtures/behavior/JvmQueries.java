class QueryChecks {
    static final Behaviors.Entity entity = new Behaviors.Entity("Product", "data", "products", "id");
    static Object result = java.util.List.of(java.util.Map.of("id", "one"));
    static int calls;
    static int authorized;
    static boolean allow = true;
    static boolean unavailable;
    static java.util.Map<String, Object> command;
    static void check(boolean valid) {
        if (!valid) {
            throw new AssertionError("query contract");
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
    static boolean authorizeQuery(java.util.Map<String, Object> request, Behaviors.Execution execution) {
        execution.check();
        authorized++;
        check(request.get("operation").equals("query"));
        return allow;
    }
    static Object executeQuery(java.util.Map<String, Object> request, Behaviors.Execution execution) {
        execution.check();
        check(authorized > calls);
        calls++;
        command = request;
        if (unavailable) {
            throw new IllegalStateException("private database details");
        }
        return result;
    }
    static Behaviors.Context context(Behaviors.Signal signal, boolean authorize) {
        Behaviors.Authorization authorization = null;
        if (authorize) {
            authorization = (request, execution) -> authorizeQuery(request, execution);
        }
        Behaviors.Options options = new Behaviors.Options("query-tenant", signal);
        java.util.Map<String, Behaviors.EntityAdapter> entities = java.util.Map.of("data", (request, execution) -> executeQuery(request, execution));
        return new Behaviors.Context(options, new Behaviors.Configuration(java.util.Map.of(), java.util.Map.of(), entities, null, authorization));
    }
    static Object query(Behaviors.Context context, String kind) {
        return query(context, kind, new Object[][]{});
    }
    static Object query(Behaviors.Context context, String kind, Object[][] where) {
        return context.query(entity, kind, new String[]{"id", "status"}, where, new String[][]{}, java.util.Map.of("size", 2), 20, 100);
    }
    static void run() {
        Behaviors.Signal signal = new Behaviors.Signal();
        Behaviors.Context context = context(signal, true);
        checkFindCommand(context);
        checkRejectedQueries(context);
        checkAcceptedBindings(context);
        checkResultShapes(context);
        checkUnavailableAdapter(context);
        checkCancellation(signal, context);
        System.out.println("Java declarative query boundary checks passed");
    }
    static void checkFindCommand(Behaviors.Context context) {
        check(query(context, "find", new Object[][]{{"status", "eq", "active"}, {"id", "isNotNull"}}).equals(result));
        check(command.get("table").equals("products") && command.get("idField").equals("id"));
        java.util.Map<String, Object> descriptor = Behaviors.map(command.get("query"));
        check(Behaviors.map(descriptor.get("page")).get("size").equals(2L));
    }
    static void checkRejectedQueries(Behaviors.Context context) {
        int callsBefore = calls;
        allow = false;
        fails("authorization", () -> query(context, "find"));
        allow = true;
        fails("authorization", () -> query(context(new Behaviors.Signal(), false), "find"));
        check(calls == callsBefore);
        fails("query-binding", () -> query(context, "find", new Object[][]{{"id", "eq", java.util.Map.of()}}));
        fails("query-binding", () -> query(context, "find", new Object[][]{{"id", "in", java.util.Collections.nCopies(101, "one")}}));
        fails("json-number", () -> query(context, "find", new Object[][]{{"id", "eq", Double.NaN}}));
        fails("json-size", () -> query(context, "find", new Object[][]{{"id", "eq", "x".repeat(40000)}, {"status", "eq", "x".repeat(40000)}}));
        fails("pagination", () -> context.query(entity, "find", new String[]{"id"}, new Object[][]{}, new String[][]{{"id", "asc"}}, java.util.Map.of("sort", java.util.List.of("id")), 20, 100));
        check(calls == callsBefore);
    }
    static void checkAcceptedBindings(Behaviors.Context context) {
        query(context, "find", new Object[][]{{"id", "eq", "x".repeat(65480)}});
        query(context, "find", new Object[][]{{"id", "in", java.util.Arrays.asList(null, "one", 2, false)}});
        for (String operator : new String[]{"eq", "ne", "lt", "lte", "gt", "gte"}) {
            query(context, "find", new Object[][]{{"id", operator, null}});
        }
    }
    static void checkResultShapes(Behaviors.Context context) {
        result = 7;
        check(query(context, "count").equals(7L));
        check(!Behaviors.map(command.get("query")).containsKey("page"));
        result = true;
        check(query(context, "exists").equals(true));
        result = -1;
        fails("query-result", () -> query(context, "count"));
        result = "true";
        fails("query-result", () -> query(context, "exists"));
        result = java.util.List.of("not-row");
        fails("query-result", () -> query(context, "find"));
        result = java.util.Collections.nCopies(3, java.util.Map.of());
        fails("query-result", () -> query(context, "find"));
        result = java.util.List.of(java.util.Map.of("id", "x".repeat(1048576)));
        fails("json-size", () -> query(context, "find"));
    }
    static void checkUnavailableAdapter(Behaviors.Context context) {
        unavailable = true;
        fails("entity-adapter", () -> query(context, "find"));
        unavailable = false;
    }
    static void checkCancellation(Behaviors.Signal signal, Behaviors.Context context) {
        int callsBefore = calls;
        signal.abort();
        fails("aborted", () -> query(context, "find"));
        check(calls == callsBefore);
        java.util.Map<String, Behaviors.EntityAdapter> entities = java.util.Map.of("data", (request, execution) -> {
            throw new AssertionError("executed after abort");
        });
        Behaviors.Authorization abortWhileAuthorizing = (request, execution) -> {
            execution.signal.abort();
            return true;
        };
        Behaviors.Options options = new Behaviors.Options("cancelled-query", new Behaviors.Signal());
        Behaviors.Configuration configuration = new Behaviors.Configuration(java.util.Map.of(), java.util.Map.of(), entities, null, abortWhileAuthorizing);
        Behaviors.Context cancelled = new Behaviors.Context(options, configuration);
        fails("aborted", () -> query(cancelled, "find"));
    }
}
