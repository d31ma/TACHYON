final class ApplicationEventPublisher {
    <T> T publishEvent(T value) { return __YonEvents.publishModel(value, false); }
    <T> T publishEvent(T value, boolean retain) { return __YonEvents.publishModel(value, retain); }
}
final class __YonEvents implements AutoCloseable {
    interface Hook { void run() throws Exception; }
    interface Listener { Object run(Object value) throws Exception; }
    private record Subscription(String name, Listener call, long order, Object condition, java.util.Map<?,?> models, String returns, int sequence) {}
    private record Event(String name, String json) {}
    private static final ThreadLocal<__YonEvents> active = new ThreadLocal<>();
    private final java.util.ArrayDeque<Event> queue = new java.util.ArrayDeque<>();
    private final java.util.Map<String,String> retained = new java.util.LinkedHashMap<>();
    private final java.util.List<Subscription> listeners = new java.util.ArrayList<>();
    final java.util.List<Hook> destroyers = new java.util.ArrayList<>();
    private final java.util.List<Event> outbound = new java.util.ArrayList<>();
    private final java.util.Set<String> topics;
    private java.util.Set<String> only;
    private Event delivered;
    private boolean closed;
    private int deliveries;
    __YonEvents(String... topics) {
        if (active.get() != null) throw new IllegalStateException("Nested local event invocation");
        this.topics = java.util.Set.of(topics);
        active.set(this);
    }
    static __YonEvents delivery(String body, String... topics) {
        var events = new __YonEvents(topics);
        var delivery = (java.util.Map<?,?>) Json.parse(body);
        events.only = new java.util.HashSet<>();
        for (Object key : (java.util.List<?>) delivery.get("listeners")) events.only.add(String.valueOf(key));
        events.delivered = new Event(String.valueOf(delivery.get("topic")), Json.stringify(delivery.get("payload")));
        return events;
    }
    void receiveDelivery() { if (delivered != null) queue.add(delivered); }
    static <T> T publish(String name, T value, boolean retain) {
        try {
        var scope = active.get();
        if (scope == null || scope.closed) throw new IllegalStateException("Local event owner is disposed");
        var json = Json.stringify(value);
        if (json.getBytes(java.nio.charset.StandardCharsets.UTF_8).length > 65536 || scope.queue.size() >= 128) throw new IllegalStateException("Local event capacity exceeded");
        if (retain) scope.retained.put(name, json);
        scope.queue.add(new Event(name,json));
        if (scope.topics.contains(name)) scope.outbound.add(new Event(name,json));
        } catch (Exception failure) { System.err.println("Yon local event publication failed."); }
        return value;
    }
    static <T> T publishModel(T value, boolean retain) {
        if (!isModel(value)) { System.err.println("Yon local event publication failed."); return value; }
        return publish(value.getClass().getSimpleName(), value, retain);
    }
    private static boolean isModel(Object value) {
        if (value == null || value.getClass().isArray()) return false;
        return !(value instanceof java.util.Map<?,?> || value instanceof Iterable<?> || value instanceof CharSequence
            || value instanceof Number || value instanceof Boolean || value instanceof Enum<?>);
    }
    static boolean fits(java.util.Map<?,?> models, String shape, Object value) {
        if (shape.startsWith("?")) return value == null || fits(models, shape.substring(1), value);
        if (shape.startsWith("[]")) {
            if (!(value instanceof java.util.List<?> items)) return false;
            for (Object item : items) if (!fits(models, shape.substring(2), item)) return false;
            return true;
        }
        switch (shape) {
            case "any": return true;
            case "string": return value instanceof String;
            case "integer": return value instanceof Long || value instanceof Integer || value instanceof java.math.BigInteger;
            case "number": return value instanceof Number;
            case "boolean": return value instanceof Boolean;
            default: break;
        }
        if (!(models.get(shape) instanceof java.util.Map<?,?> model) || !(value instanceof java.util.Map<?,?> object)) return false;
        var fields = (java.util.Map<?,?>) model.get("fields");
        if (!Boolean.TRUE.equals(model.get("open"))) for (Object key : object.keySet()) if (!fields.containsKey(key)) return false;
        for (var entry : fields.entrySet()) {
            String field = String.valueOf(entry.getValue());
            boolean required = field.startsWith("!");
            if (!object.containsKey(entry.getKey())) { if (required) return false; continue; }
            if (!fits(models, required ? field.substring(1) : field, object.get(entry.getKey()))) return false;
        }
        return true;
    }
    static boolean matches(Object matcher, Object value, String event) {
        try { return test((java.util.Map<?,?>) matcher, value, event); } catch (Exception failure) { return false; }
    }
    private static Object operand(java.util.Map<?,?> node, Object value, String event) {
        var kind = node.get("k");
        if ("lit".equals(kind)) return node.get("v");
        if ("event".equals(kind)) return event;
        Object current = value;
        var path = node.get("path") instanceof java.util.List<?> segments ? segments : java.util.List.of();
        for (Object segment : path) {
            if (current instanceof java.util.Map<?,?> map) current = map.get(String.valueOf(segment));
            else if (current instanceof java.util.List<?> list && segment instanceof Number index && index.intValue() >= 0 && index.intValue() < list.size()) current = list.get(index.intValue());
            else return null;
        }
        return current;
    }
    private static boolean truthy(Object value) {
        if (value == null) return false;
        if (value instanceof Boolean flag) return flag;
        if (value instanceof Number number) return number.doubleValue() != 0;
        if (value instanceof String text) return !text.isEmpty();
        return true;
    }
    private static boolean test(java.util.Map<?,?> node, Object value, String event) {
        return switch (String.valueOf(node.get("k"))) {
            case "and" -> test((java.util.Map<?,?>) node.get("l"), value, event) && test((java.util.Map<?,?>) node.get("r"), value, event);
            case "or" -> test((java.util.Map<?,?>) node.get("l"), value, event) || test((java.util.Map<?,?>) node.get("r"), value, event);
            case "not" -> !test((java.util.Map<?,?>) node.get("e"), value, event);
            case "truthy" -> truthy(operand((java.util.Map<?,?>) node.get("e"), value, event));
            case "cmp" -> compare(String.valueOf(node.get("op")), operand((java.util.Map<?,?>) node.get("l"), value, event), operand((java.util.Map<?,?>) node.get("r"), value, event));
            default -> false;
        };
    }
    private static boolean compare(String op, Object left, Object right) {
        boolean numbers = left instanceof Number && right instanceof Number;
        boolean strings = left instanceof String && right instanceof String;
        if (op.equals("eq") || op.equals("ne")) {
            boolean equal = numbers ? ((Number) left).doubleValue() == ((Number) right).doubleValue()
                : (strings || left instanceof Boolean) ? left.equals(right)
                : left == null && right == null;
            return op.equals("eq") == equal;
        }
        int order;
        if (numbers) order = Double.compare(((Number) left).doubleValue(), ((Number) right).doubleValue());
        else if (strings) order = ((String) left).compareTo((String) right);
        else return false;
        return switch (op) { case "lt" -> order < 0; case "le" -> order <= 0; case "gt" -> order > 0; default -> order >= 0; };
    }
    void listen(String options, Listener callback, boolean replay, String key) {
        if (only != null && !only.contains(key)) return;
        var compiled = (java.util.Map<?,?>) Json.parse(options);
        long order = compiled.get("order") instanceof Number number ? number.longValue() : 0;
        var models = compiled.get("models") instanceof java.util.Map<?,?> declared ? declared : null;
        var returns = compiled.get("returns") instanceof String returned ? returned : null;
        for (Object name : (java.util.List<?>) compiled.get("events")) {
            if (listeners.size() >= 1024) throw new IllegalStateException("Local listener capacity exceeded");
            listeners.add(new Subscription(String.valueOf(name), callback, order, compiled.get("condition"), models, returns, listeners.size()));
            if (replay && retained.containsKey(String.valueOf(name))) queue.add(new Event(String.valueOf(name), retained.get(String.valueOf(name))));
        }
        listeners.sort(java.util.Comparator.comparingLong(Subscription::order).thenComparingInt(Subscription::sequence));
    }
    private static void republish(Object result, String returns) {
        if (returns == null) return;
        var items = result instanceof java.util.List<?> list ? list : result == null ? java.util.List.of() : java.util.List.of(result);
        for (Object item : items.subList(0, Math.min(items.size(), 32))) {
            if (item != null) publish(returns, item, false);
        }
    }
    void drain() {
        while (!queue.isEmpty()) {
            var event = queue.remove();
            if (++deliveries > 4096) { queue.clear(); System.err.println("Yon local event delivery bound exceeded."); return; }
            for (var listener : listeners) if (listener.name.equals(event.name)) {
                var value = Json.parse(event.json);
                if (listener.models != null && !fits(listener.models, event.name, value)) { System.err.println("Yon local event payload does not fit the listener model."); continue; }
                if (listener.condition != null && !matches(listener.condition, value, event.name)) continue;
                try { republish(listener.call.run(value), listener.returns); }
                catch (Exception failure) { System.err.println("Yon local event listener failed."); }
            }
        }
        sendTopics();
    }
    private void sendTopics() {
        for (var event : outbound) {
            try { YonManagedResources.call(YonManagedResources.command("operation", "topic", "topic", event.name, "payload_json", event.json)); }
            catch (RuntimeException failure) { System.err.println("Yon topic publication failed."); }
        }
        outbound.clear();
    }
    public void close() {
        if (closed) return; closed = true; queue.clear(); listeners.clear();
        for (int index = destroyers.size()-1; index >= 0; index--) {
            try { destroyers.get(index).run(); } catch (Exception failure) { System.err.println("Yon local lifecycle cleanup failed."); }
        }
        retained.clear(); active.remove();
    }
}
