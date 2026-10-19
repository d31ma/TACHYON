final class YonLauncher {
    public static void main(String[] args) throws java.io.IOException {
        Yon.main(args);
    }
}

final class DataSource {
    Object query(String sql, java.util.List<?> parameters) {
        return YonManagedResources.call(YonManagedResources.command("operation", "sql", "sql", sql, "parameters", parameters, "mode", "rows"));
    }
    Object execute(String sql, java.util.List<?> parameters) {
        return YonManagedResources.call(YonManagedResources.command("operation", "sql", "sql", sql, "parameters", parameters, "mode", "execute"));
    }
    Object execute(java.util.Map<String, ?> command) { return command(command); }
    Object command(java.util.Map<String, ?> command) { return YonManagedResources.call(command); }
    Object findById(String collection, String id) { return document("findById", "collection", collection, "id", id); }
    Object insert(String collection, java.util.Map<String, ?> document) { return document("insert", "collection", collection, "value", document); }
    Object patch(String collection, String id, java.util.Map<String, ?> fields) { return document("patch", "collection", collection, "id", id, "value", fields); }
    Object deleteById(String collection, String id) { return document("deleteById", "collection", collection, "id", id); }
    Object scroll(String collection, java.util.Map<String, ?> where, String cursor, int size) {
        var command = YonManagedResources.command("operation", "document", "command", "scroll", "collection", collection, "where", where, "cursor", cursor, "size", size);
        return YonManagedResources.call(command, YonManagedResources.SCROLL_TIMEOUT_NANOSECONDS);
    }
    Object createCollection(String name) { return document("createCollection", "collection", name); }
    Object dropCollection(String name) { return document("dropCollection", "collection", name); }
    Object collectionExists(String name) { return document("collectionExists", "collection", name); }
    Object publish(String topic, Object payload, String key) { return queue("publish", "topic", topic, "payload", payload, "key", key); }
    Object claim(String topic, String group, int maxMessages, int leaseMs) { return queue("claim", "topic", topic, "group", group, "maxMessages", maxMessages, "leaseMs", leaseMs); }
    Object ack(String topic, String group, java.util.Map<?, ?> delivery) { return queue("ack", "topic", topic, "group", group, "id", delivery.get("id"), "receipt", delivery.get("receipt")); }
    Object nack(String topic, String group, java.util.Map<?, ?> delivery, int delayMs) {
        return queue("nack", "topic", topic, "group", group, "id", delivery.get("id"), "receipt", delivery.get("receipt"), "delayMs", delayMs);
    }
    private static Object document(String command, Object... fields) { return YonManagedResources.call(YonManagedResources.tagged("document", command, fields)); }
    private static Object queue(String command, Object... fields) { return YonManagedResources.call(YonManagedResources.tagged("queue", command, fields)); }
}

final class RedisTemplate {
    Object get(String key) { return YonManagedResources.call(YonManagedResources.command("operation", "redis", "command", "get", "key", key)); }
    Object delete(String key) { return YonManagedResources.call(YonManagedResources.command("operation", "redis", "command", "delete", "key", key)); }
    Object set(String key, Object value, long ttlSeconds) {
        var command = new java.util.LinkedHashMap<String, Object>();
        command.put("operation", "redis"); command.put("command", "set");
        command.put("key", key); command.put("value", value); command.put("ttlSeconds", ttlSeconds);
        return YonManagedResources.call(command);
    }
}

final class YonManagedResources {
    static final int LIMIT = 1024 * 1024;
    private static final long RESOURCE_TIMEOUT_NANOSECONDS = 5_000_000_000L;
    static final long SCROLL_TIMEOUT_NANOSECONDS = 30_000_000_000L;
    private static final long CONNECT_TIMEOUT_MILLISECONDS = 5000L;
    private static IllegalStateException failed() { return new IllegalStateException("Yon managed resource operation failed."); }
    static java.util.Map<String, Object> command(Object... fields) {
        var command = new java.util.LinkedHashMap<String, Object>();
        for (int index = 0; index < fields.length; index += 2) command.put((String) fields[index], fields[index + 1]);
        return command;
    }
    static java.util.Map<String, Object> tagged(String operation, String command, Object... fields) {
        var tagged = command(fields);
        tagged.put("operation", operation);
        tagged.put("command", command);
        return tagged;
    }
    static Object call(Object command) {
        return call(command, RESOURCE_TIMEOUT_NANOSECONDS);
    }
    static Object call(Object command, long timeoutNanoseconds) {
        String text;
        try {
            text = Json.stringify(command);
        } catch (RuntimeException invalid) {
            throw failed();
        }
        return exchange(text, timeoutNanoseconds);
    }
    static Object exchange(String text, long timeoutNanoseconds) {
        try {
            String endpoint = System.getenv("YON_RESOURCE_ENDPOINT");
            String token = System.getenv("YON_RESOURCE_TOKEN");
            if (endpoint == null || !endpoint.matches("http://127\\.0\\.0\\.1:[0-9]{1,5}/") || token == null || !token.matches("[0-9a-f]{64}")) throw failed();
            int port = Integer.parseInt(endpoint.substring(17, endpoint.length() - 1));
            if (port < 1 || port > 65535) throw failed();
            byte[] body = text.getBytes(java.nio.charset.StandardCharsets.UTF_8);
            if (body.length > LIMIT) throw failed();
            long deadline = System.nanoTime() + timeoutNanoseconds;
            int connectTimeout = (int) Math.min(CONNECT_TIMEOUT_MILLISECONDS, timeoutNanoseconds / 1_000_000L);
            try (var socket = new java.net.Socket()) {
                socket.connect(new java.net.InetSocketAddress("127.0.0.1", port), connectTimeout);
                String headers = "POST / HTTP/1.1\r\nHost: 127.0.0.1:" + port + "\r\nAuthorization: Bearer " + token + "\r\nContent-Type: application/json\r\nContent-Length: " + body.length + "\r\nConnection: close\r\n\r\n";
                var output = socket.getOutputStream();
                output.write(headers.getBytes(java.nio.charset.StandardCharsets.US_ASCII));
                output.write(body); output.flush();
                var input = socket.getInputStream();
                var head = new java.io.ByteArrayOutputStream();
                int matched = 0;
                byte[] delimiter = {13, 10, 13, 10};
                while (matched < 4) {
                    timeout(socket, deadline);
                    int next = input.read();
                    if (next < 0 || head.size() >= 16384) throw failed();
                    head.write(next);
                    matched = next == delimiter[matched] ? matched + 1 : (next == 13 ? 1 : 0);
                }
                String[] lines = head.toString(java.nio.charset.StandardCharsets.US_ASCII).split("\r\n");
                if (!lines[0].matches("HTTP/1\\.[01] 200(?: .*)?")) throw failed();
                int length = -1;
                for (int index = 1; index < lines.length; index++) {
                    String line = lines[index].toLowerCase(java.util.Locale.ROOT);
                    if (line.startsWith("transfer-encoding:")) throw failed();
                    if (line.startsWith("content-length:")) {
                        String value = line.substring(15).trim();
                        if (length >= 0 || !value.matches("[0-9]{1,7}")) throw failed();
                        length = Integer.parseInt(value);
                    }
                }
                if (length < 0 || length > LIMIT) throw failed();
                byte[] response = new byte[length];
                int read = 0;
                while (read < length) {
                    timeout(socket, deadline);
                    int count = input.read(response, read, length - read);
                    if (count <= 0) throw failed();
                    read += count;
                }
                return Json.parse(java.nio.charset.StandardCharsets.UTF_8.newDecoder().decode(java.nio.ByteBuffer.wrap(response)).toString());
            }
        } catch (Exception ignored) { throw failed(); }
    }
    private static void timeout(java.net.Socket socket, long deadline) throws java.io.IOException {
        long remaining = (deadline - System.nanoTime()) / 1_000_000L;
        if (remaining <= 0) throw failed();
        socket.setSoTimeout((int) Math.min(Integer.MAX_VALUE, remaining));
    }
}

final class YonGateway {
    private static final long ANSWER_MARGIN_NANOSECONDS = 1_000_000_000L;
    static java.util.Map<?, ?> call(String key, Object[] arguments) {
        String body;
        try {
            body = Json.stringify(YonManagedResources.command("operation", "gateway", "key", key, "arguments", arguments));
        } catch (RuntimeException invalid) {
            return failure("invalid");
        }
        if (body.getBytes(java.nio.charset.StandardCharsets.UTF_8).length > YonManagedResources.LIMIT) return failure("invalid");
        long timeLeft = Yon.timeLeftNanos();
        if (timeLeft <= 0) return failure("timeout");
        Object answer;
        try {
            answer = YonManagedResources.exchange(body, timeLeft + ANSWER_MARGIN_NANOSECONDS);
        } catch (RuntimeException failed) {
            if (Yon.timeLeftNanos() <= 0) return failure("timeout");
            return failure("adapter");
        }
        if (!(answer instanceof java.util.Map<?, ?> map)) return failure("adapter");
        if (map.containsKey("result") || map.get("error") instanceof String) return map;
        return failure("adapter");
    }
    private static java.util.Map<String, Object> failure(String kind) {
        return java.util.Map.of("error", kind);
    }
}

final class Json {
    private static final int MAX_CHARS = 16 * 1024 * 1024;
    private static final int MAX_DEPTH = 64;
    private static final java.util.regex.Pattern NUMBER =
        java.util.regex.Pattern.compile("-?(?:0|[1-9][0-9]*)(?:\\.[0-9]+)?(?:[eE][+-]?[0-9]+)?");
    private final String source;
    private int at;
    private Json(String source) { this.source = source; }

    static Object parse(String source) {
        if (source == null || source.length() > MAX_CHARS) throw invalid();
        Json reader = new Json(source);
        Object value = reader.value(0);
        reader.skip();
        if (reader.at != source.length()) throw invalid();
        return value;
    }

    static String stringify(Object value) {
        StringBuilder out = new StringBuilder();
        append(out, value, 0);
        return out.toString();
    }
    static String write(Object value) { return stringify(value); }
    static String quote(String text) {
        StringBuilder out = new StringBuilder();
        quoted(out, text);
        return out.toString();
    }
    private static IllegalArgumentException invalid() {
        return new IllegalArgumentException("Invalid or unsupported JSON value.");
    }
    private static void check(StringBuilder out) {
        if (out.length() > MAX_CHARS) throw invalid();
    }
    private static void append(StringBuilder out, Object value, int depth) {
        if (depth > MAX_DEPTH) throw invalid();
        if (value == null) out.append("null");
        else if (value instanceof String text) quoted(out, text);
        else if (value instanceof Boolean flag) out.append(flag);
        else if (value instanceof Number number) {
            String text = number.toString();
            if (!NUMBER.matcher(text).matches()) throw invalid();
            out.append(text);
        } else if (value instanceof java.util.Map<?, ?> map) {
            out.append('{');
            boolean first = true;
            for (var entry : map.entrySet()) {
                if (!(entry.getKey() instanceof String key)) throw invalid();
                if (!first) out.append(',');
                first = false;
                quoted(out, key);
                out.append(':');
                append(out, entry.getValue(), depth + 1);
            }
            out.append('}');
        } else if (value instanceof Iterable<?> items) {
            out.append('[');
            boolean first = true;
            for (Object item : items) {
                if (!first) out.append(',');
                first = false;
                append(out, item, depth + 1);
            }
            out.append(']');
        } else if (value.getClass().isArray()) {
            out.append('[');
            int length = java.lang.reflect.Array.getLength(value);
            for (int index = 0; index < length; index++) {
                if (index != 0) out.append(',');
                append(out, java.lang.reflect.Array.get(value, index), depth + 1);
            }
            out.append(']');
        } else if (value instanceof Enum<?> item) quoted(out, item.name());
        else if (value instanceof java.util.Optional<?> optional) append(out, optional.orElse(null), depth + 1);
        else if (value instanceof __YonAnswer || value instanceof Class<?>) throw invalid();
        else append(out, YonModels.plain(value), depth + 1);
        check(out);
    }
    private static void quoted(StringBuilder out, String text) {
        out.append('"');
        for (int index = 0; index < text.length(); index++) {
            char c = text.charAt(index);
            switch (c) {
                case '"' -> out.append("\\\"");
                case '\\' -> out.append("\\\\");
                case '\n' -> out.append("\\n");
                case '\r' -> out.append("\\r");
                case '\t' -> out.append("\\t");
                default -> {
                    if (c < 0x20 || Character.isSurrogate(c))
                        out.append(String.format("\\u%04x", (int)c));
                    else out.append(c);
                }
            }
            check(out);
        }
        out.append('"');
    }
    private void skip() {
        while (at < source.length() && " \n\r\t".indexOf(source.charAt(at)) >= 0) at++;
    }
    private boolean take(char expected) {
        skip();
        if (at >= source.length() || source.charAt(at) != expected) return false;
        at++;
        return true;
    }
    private void require(char expected) { if (!take(expected)) throw invalid(); }
    private Object literal(String text, Object value) {
        if (!source.startsWith(text, at)) throw invalid();
        at += text.length();
        return value;
    }
    private Object value(int depth) {
        skip();
        if (depth > MAX_DEPTH || at >= source.length()) throw invalid();
        return switch (source.charAt(at)) {
            case '{' -> object(depth);
            case '[' -> array(depth);
            case '"' -> string();
            case 't' -> literal("true", Boolean.TRUE);
            case 'f' -> literal("false", Boolean.FALSE);
            case 'n' -> literal("null", null);
            default -> number();
        };
    }
    private java.util.Map<String, Object> object(int depth) {
        require('{');
        var entries = new java.util.LinkedHashMap<String, Object>();
        if (take('}')) return entries;
        do {
            skip();
            String key = string();
            require(':');
            entries.put(key, value(depth + 1));
            if (take('}')) return entries;
            require(',');
        } while (true);
    }
    private java.util.List<Object> array(int depth) {
        require('[');
        var values = new java.util.ArrayList<Object>();
        if (take(']')) return values;
        do {
            values.add(value(depth + 1));
            if (take(']')) return values;
            require(',');
        } while (true);
    }
    private String string() {
        require('"');
        var out = new StringBuilder();
        while (at < source.length()) {
            char c = source.charAt(at++);
            if (c == '"') return out.toString();
            if (c < 0x20) throw invalid();
            if (c != '\\') { out.append(c); continue; }
            if (at >= source.length()) throw invalid();
            char escaped = source.charAt(at++);
            switch (escaped) {
                case '"', '\\', '/' -> out.append(escaped);
                case 'n' -> out.append('\n');
                case 'r' -> out.append('\r');
                case 't' -> out.append('\t');
                case 'b' -> out.append('\b');
                case 'f' -> out.append('\f');
                case 'u' -> {
                    if (at + 4 > source.length()) throw invalid();
                    String digits = source.substring(at, at + 4);
                    if (!digits.matches("[0-9a-fA-F]{4}")) throw invalid();
                    out.append((char)Integer.parseInt(digits, 16));
                    at += 4;
                }
                default -> throw invalid();
            }
        }
        throw invalid();
    }
    private Object number() {
        var match = NUMBER.matcher(source);
        match.region(at, source.length());
        if (!match.lookingAt()) throw invalid();
        String digits = match.group();
        at = match.end();
        try { return Long.valueOf(digits); }
        catch (NumberFormatException fractional) { return new java.math.BigDecimal(digits); }
    }
}

final class __YonFrame {
    private final java.util.Map<String, Object> raw;

    @SuppressWarnings("unchecked")
    __YonFrame(Object parsed) {
        this.raw = parsed instanceof java.util.Map ? (java.util.Map<String, Object>) parsed : java.util.Map.of();
    }

    public String method() {
        return String.valueOf(raw.getOrDefault("method", ""));
    }

    public String route() {
        return String.valueOf(raw.getOrDefault("route", ""));
    }

    @SuppressWarnings("unchecked")
    public String parameter(String name) {
        Object parameters = raw.get("parameters");
        if (!(parameters instanceof java.util.Map)) return "";
        Object value = ((java.util.Map<String, Object>) parameters).get(name);
        return value == null ? "" : String.valueOf(value);
    }

    @SuppressWarnings("unchecked")
    public String body() {
        Object body = raw.get("body");
        if (!(body instanceof java.util.Map)) return "";
        Object data = ((java.util.Map<String, Object>) body).get("data");
        return data == null ? "" : String.valueOf(data);
    }

    public java.util.Map<String, Object> raw() {
        return raw;
    }

    @SuppressWarnings("unchecked")
    java.util.Map<String, java.util.List<String>> headers() {
        if (!(raw.get("headers") instanceof java.util.Map<?, ?> headers)) return java.util.Map.of();
        return (java.util.Map<String, java.util.List<String>>) headers;
    }

    String target() {
        if (raw.get("target") instanceof String target) return target;
        return route();
    }

    @SuppressWarnings("unchecked")
    public java.util.List<Object> arguments() {
        Object arguments = raw.get("arguments");
        if (!(arguments instanceof java.util.List)) return java.util.List.of();
        return (java.util.List<Object>) arguments;
    }
}

final class YonModels {
    static Object argument(Class<?> owner, String method, int count, int index, Object value) throws Exception {
        for (java.lang.reflect.Method candidate : owner.getDeclaredMethods()) {
            if (candidate.getName().equals(method) && candidate.getParameterCount() == count) {
                return read(candidate.getGenericParameterTypes()[index], value);
            }
        }
        return value;
    }

    static Object read(java.lang.reflect.Type type, Object value) throws Exception {
        if (type instanceof java.lang.reflect.ParameterizedType generic) {
            return readGeneric(generic, value);
        }
        if (!(type instanceof Class<?> kind)) return value;
        if (value == null) return zero(kind);
        if (value instanceof Number number) return number(kind, number);
        if (kind.isEnum()) return enumValue(kind, value);
        if (kind.isArray() && value instanceof java.util.List<?> list) return array(kind, list);
        if (!(value instanceof java.util.Map<?, ?> map) || kind == Object.class) return value;
        if (java.util.Map.class.isAssignableFrom(kind)) return value;
        if (kind.isRecord()) return record(kind, map);
        return fields(kind, map);
    }

    private static Object readGeneric(java.lang.reflect.ParameterizedType generic, Object value) throws Exception {
        Class<?> raw = (Class<?>) generic.getRawType();
        java.lang.reflect.Type[] items = generic.getActualTypeArguments();
        if (raw == java.util.Optional.class) return java.util.Optional.ofNullable(value == null ? null : read(items[0], value));
        if (value == null) return null;
        if (!java.util.Collection.class.isAssignableFrom(raw) || !(value instanceof java.util.List<?> list)) return value;
        java.util.List<Object> result = new java.util.ArrayList<>();
        for (Object item : list) result.add(read(items[0], item));
        if (java.util.Set.class.isAssignableFrom(raw)) return new java.util.LinkedHashSet<>(result);
        return result;
    }

    private static Object zero(Class<?> kind) {
        if (kind == boolean.class) return false;
        if (kind == char.class) return '\0';
        if (kind.isPrimitive()) return number(kind, 0L);
        return null;
    }

    private static Object number(Class<?> kind, Number number) {
        if (kind == int.class || kind == Integer.class) return number.intValue();
        if (kind == long.class || kind == Long.class) return number.longValue();
        if (kind == short.class || kind == Short.class) return number.shortValue();
        if (kind == byte.class || kind == Byte.class) return number.byteValue();
        if (kind == double.class || kind == Double.class) return number.doubleValue();
        if (kind == float.class || kind == Float.class) return number.floatValue();
        if (kind == java.math.BigDecimal.class) return new java.math.BigDecimal(number.toString());
        if (kind == java.math.BigInteger.class) return new java.math.BigInteger(number.toString());
        return number;
    }

    @SuppressWarnings({"unchecked", "rawtypes"})
    private static Object enumValue(Class<?> kind, Object value) {
        return Enum.valueOf((Class) kind, String.valueOf(value));
    }

    private static Object array(Class<?> kind, java.util.List<?> list) throws Exception {
        Object array = java.lang.reflect.Array.newInstance(kind.getComponentType(), list.size());
        for (int index = 0; index < list.size(); index++) {
            java.lang.reflect.Array.set(array, index, read(kind.getComponentType(), list.get(index)));
        }
        return array;
    }

    private static Object record(Class<?> kind, java.util.Map<?, ?> map) throws Exception {
        java.lang.reflect.RecordComponent[] components = kind.getRecordComponents();
        Object[] values = new Object[components.length];
        Class<?>[] types = new Class<?>[components.length];
        for (int index = 0; index < components.length; index++) {
            types[index] = components[index].getType();
            values[index] = read(components[index].getGenericType(), map.get(components[index].getName()));
        }
        java.lang.reflect.Constructor<?> constructor = kind.getDeclaredConstructor(types);
        constructor.setAccessible(true);
        return constructor.newInstance(values);
    }

    private static Object fields(Class<?> kind, java.util.Map<?, ?> map) throws Exception {
        java.lang.reflect.Constructor<?> constructor = kind.getDeclaredConstructor();
        constructor.setAccessible(true);
        Object model = constructor.newInstance();
        for (java.lang.reflect.Field field : instanceFields(kind)) {
            if (!map.containsKey(field.getName())) continue;
            field.setAccessible(true);
            field.set(model, read(field.getGenericType(), map.get(field.getName())));
        }
        return model;
    }

    private static java.util.List<java.lang.reflect.Field> instanceFields(Class<?> kind) {
        java.util.List<java.lang.reflect.Field> fields = new java.util.ArrayList<>();
        for (Class<?> current = kind; current != null && current != Object.class; current = current.getSuperclass()) {
            java.util.List<java.lang.reflect.Field> own = new java.util.ArrayList<>();
            for (java.lang.reflect.Field field : current.getDeclaredFields()) {
                int modifiers = field.getModifiers();
                boolean hidden = java.lang.reflect.Modifier.isStatic(modifiers) || java.lang.reflect.Modifier.isTransient(modifiers) || field.isSynthetic();
                if (!hidden) own.add(field);
            }
            fields.addAll(0, own);
        }
        return fields;
    }

    static java.util.Map<String, Object> plain(Object model) {
        java.util.Map<String, Object> fields = new java.util.LinkedHashMap<>();
        try {
            if (model.getClass().isRecord()) {
                for (java.lang.reflect.RecordComponent component : model.getClass().getRecordComponents()) {
                    java.lang.reflect.Method accessor = component.getAccessor();
                    accessor.setAccessible(true);
                    fields.put(component.getName(), accessor.invoke(model));
                }
                return fields;
            }
            for (java.lang.reflect.Field field : instanceFields(model.getClass())) {
                field.setAccessible(true);
                fields.put(field.getName(), field.get(model));
            }
        } catch (ReflectiveOperationException failed) {
            throw new IllegalArgumentException("Invalid or unsupported JSON value.");
        }
        return fields;
    }
}

interface YonSocketEndpoint {
    void open(SocketSession session) throws Exception;
    void message(SocketSession session, Object value) throws Exception;
    void close(SocketSession session) throws Exception;
    void error(SocketSession session, Exception error) throws Exception;
}

interface YonSocketAction {
    void run() throws Exception;
}

final class SocketPeer {
    private final SocketSession session;
    private final String id;

    SocketPeer(SocketSession session, String id) {
        this.session = session;
        this.id = id;
    }

    public String getId() { return id; }

    public void send(Object message) throws java.io.IOException { session.send(message, id); }
}

final class SocketSession {
    static final Object ENDED = new Object();
    private final String id;
    private final java.util.ArrayDeque<Object> inbox = new java.util.ArrayDeque<>();
    private boolean closed;
    private java.util.List<?> sessions;

    SocketSession(String id) { this.id = id; }

    public String getId() { return id; }

    public void send(Object message) throws java.io.IOException { send(message, null); }

    public void close() throws java.io.IOException {
        closed = true;
        write(java.util.Map.of("kind", "close"));
    }

    public java.util.List<SocketPeer> getOpenSessions() throws java.io.IOException {
        sessions = null;
        write(java.util.Map.of("kind", "sessions"));
        while (sessions == null && !closed) receive();
        var peers = new java.util.ArrayList<SocketPeer>();
        if (sessions == null) return peers;
        for (Object peer : sessions) peers.add(new SocketPeer(this, String.valueOf(peer)));
        return peers;
    }

    void send(Object message, String to) throws java.io.IOException {
        var body = java.util.Map.of("encoding", "utf8", "data", Json.stringify(message));
        var frame = new java.util.HashMap<String, Object>();
        frame.put("kind", "event");
        frame.put("body", body);
        if (to != null) frame.put("to", to);
        write(frame);
    }

    Object next() {
        while (inbox.isEmpty() && !closed) receive();
        if (inbox.isEmpty()) return ENDED;
        return inbox.removeFirst();
    }

    private void receive() {
        String raw = YonSocket.readFrame();
        if (raw == null || !(Json.parse(raw) instanceof java.util.Map<?, ?> frame)) {
            closed = true;
            return;
        }
        if (!id.equals(frame.get("request_id"))) System.exit(70);
        Object kind = frame.get("kind");
        if ("message".equals(kind) && frame.get("body") instanceof java.util.Map<?, ?> body) {
            inbox.addLast(Json.parse(String.valueOf(body.get("data"))));
        } else if ("sessions".equals(kind)) {
            sessions = frame.get("sessions") instanceof java.util.List<?> list ? list : java.util.List.of();
        } else {
            closed = true;
        }
    }

    private void write(java.util.Map<String, Object> fields) throws java.io.IOException {
        var frame = new java.util.HashMap<String, Object>(fields);
        frame.put("protocol_version", 1);
        frame.put("request_id", id);
        YonSocket.writeFrame(Json.stringify(frame));
    }
}

final class YonStream {
    private static final int MAX_FRAME_BYTES = 16 * 1024 * 1024;

    static void send(String requestId, Object result) {
        try {
            if (result instanceof java.util.stream.Stream<?> stream) {
                try (stream) {
                    send(requestId, stream.iterator());
                }
            } else {
                send(requestId, (java.util.Iterator<?>) result);
            }
        } catch (Throwable failed) {
            failure(requestId);
        }
    }

    private static void send(String requestId, java.util.Iterator<?> events) throws java.io.IOException {
        while (events.hasNext()) {
            if (!event(requestId, events.next())) return;
        }
    }

    private static boolean event(String requestId, Object value) throws java.io.IOException {
        var body = new java.util.LinkedHashMap<String, Object>();
        body.put("encoding", "utf8");
        body.put("data", Json.stringify(value));
        var frame = new java.util.LinkedHashMap<String, Object>();
        frame.put("protocol_version", 1);
        frame.put("kind", "event");
        frame.put("request_id", requestId);
        frame.put("body", body);
        byte[] payload = Json.stringify(frame).getBytes(java.nio.charset.StandardCharsets.UTF_8);
        if (payload.length > MAX_FRAME_BYTES) {
            write(refusal(requestId, "TY2203", "Streamed event exceeds the protocol frame limit."));
            return false;
        }
        write(payload);
        return true;
    }

    private static void failure(String requestId) {
        try {
            write(refusal(requestId, "TY2204", "The handler stream failed."));
        } catch (java.io.IOException closed) {
            System.exit(1);
        }
    }

    private static byte[] refusal(String requestId, String code, String message) {
        var error = new java.util.LinkedHashMap<String, Object>();
        error.put("code", code);
        error.put("message", message);
        error.put("retryable", false);
        var frame = new java.util.LinkedHashMap<String, Object>();
        frame.put("protocol_version", 1);
        frame.put("kind", "response");
        frame.put("request_id", requestId);
        frame.put("status", 500);
        frame.put("headers", new java.util.LinkedHashMap<String, Object>());
        frame.put("error", error);
        return Json.stringify(frame).getBytes(java.nio.charset.StandardCharsets.UTF_8);
    }

    private static void write(byte[] payload) throws java.io.IOException {
        var output = new java.io.DataOutputStream(System.out);
        output.writeInt(payload.length);
        output.write(payload);
        output.flush();
    }
}

final class YonSocket {
    private static final int MAX_FRAME_BYTES = 16 * 1024 * 1024;
    private static final java.io.DataInputStream INPUT = new java.io.DataInputStream(System.in);
    private static final java.io.DataOutputStream OUTPUT = new java.io.DataOutputStream(System.out);

    static boolean framed() {
        return "1".equals(System.getenv("YON_SOCKET_FRAMES"));
    }

    static String readFrame() {
        try {
            int length = INPUT.readInt();
            if (length < 0 || length > MAX_FRAME_BYTES) System.exit(70);
            byte[] payload = new byte[length];
            INPUT.readFully(payload);
            return new String(payload, java.nio.charset.StandardCharsets.UTF_8);
        } catch (java.io.IOException ended) {
            return null;
        }
    }

    static void writeFrame(String text) throws java.io.IOException {
        byte[] payload = text.getBytes(java.nio.charset.StandardCharsets.UTF_8);
        OUTPUT.writeInt(payload.length);
        OUTPUT.write(payload);
        OUTPUT.flush();
    }

    static Object run(YonSocketEndpoint endpoint, __YonFrame request) {
        var session = new SocketSession(String.valueOf(request.raw().get("request_id")));
        attempt(endpoint, session, () -> endpoint.open(session));
        while (true) {
            Object message = session.next();
            if (message == SocketSession.ENDED) break;
            attempt(endpoint, session, () -> endpoint.message(session, message));
        }
        try {
            call(() -> endpoint.close(session));
        } catch (Exception ignored) {
        }
        System.exit(0);
        return null;
    }

    private static void call(YonSocketAction action) throws Exception {
        Yon.renewDeadline();
        action.run();
    }

    private static void attempt(YonSocketEndpoint endpoint, SocketSession session, YonSocketAction action) {
        try {
            call(action);
        } catch (Exception failure) {
            var event = java.util.Map.of(
                "event", "handler.socket_failed",
                "request_id", session.getId(),
                "message", String.valueOf(failure.getMessage()));
            System.err.println(Json.stringify(event));
            try {
                call(() -> endpoint.error(session, failure));
            } catch (Exception ignored) {
            }
        }
    }
}

final class YonExchange extends com.sun.net.httpserver.HttpExchange {
    private final com.sun.net.httpserver.Headers requestHeaders = new com.sun.net.httpserver.Headers();
    private final com.sun.net.httpserver.Headers responseHeaders = new com.sun.net.httpserver.Headers();
    private final java.io.ByteArrayOutputStream written = new java.io.ByteArrayOutputStream();
    private final java.util.Map<String, Object> attributes = new java.util.HashMap<>();
    private final java.net.URI uri;
    private final String method;
    private final byte[] body;
    private int responseCode = -1;

    YonExchange(__YonFrame frame) {
        for (var header : frame.headers().entrySet()) {
            for (String value : header.getValue()) requestHeaders.add(header.getKey(), value);
        }
        uri = java.net.URI.create(frame.target());
        method = frame.method();
        body = frame.body().getBytes(java.nio.charset.StandardCharsets.UTF_8);
    }

    @Override public com.sun.net.httpserver.Headers getRequestHeaders() { return requestHeaders; }
    @Override public com.sun.net.httpserver.Headers getResponseHeaders() { return responseHeaders; }
    @Override public java.net.URI getRequestURI() { return uri; }
    @Override public String getRequestMethod() { return method; }
    @Override public com.sun.net.httpserver.HttpContext getHttpContext() { return null; }
    @Override public void close() {}
    @Override public java.io.InputStream getRequestBody() { return new java.io.ByteArrayInputStream(body); }
    @Override public java.io.OutputStream getResponseBody() { return written; }
    @Override public void sendResponseHeaders(int code, long length) { responseCode = code; }
    @Override public java.net.InetSocketAddress getRemoteAddress() { return new java.net.InetSocketAddress(clientAddress(), 0); }
    private java.net.InetAddress clientAddress() {
        String address = requestHeaders.getFirst("X-Real-IP");
        if (address == null) return java.net.InetAddress.getLoopbackAddress();
        try { return java.net.InetAddress.getByName(address); } catch (java.net.UnknownHostException error) { return java.net.InetAddress.getLoopbackAddress(); }
    }
    @Override public int getResponseCode() { return responseCode; }
    @Override public java.net.InetSocketAddress getLocalAddress() { return new java.net.InetSocketAddress(java.net.InetAddress.getLoopbackAddress(), 0); }
    @Override public String getProtocol() { return "HTTP/1.1"; }
    @Override public Object getAttribute(String name) { return attributes.get(name); }
    @Override public void setAttribute(String name, Object value) { attributes.put(name, value); }
    @Override public void setStreams(java.io.InputStream input, java.io.OutputStream output) {}
    @Override public com.sun.net.httpserver.HttpPrincipal getPrincipal() { return null; }

    boolean answered() {
        return responseCode >= 0;
    }

    __YonAnswer answer() {
        var headers = new java.util.LinkedHashMap<String, java.util.List<String>>();
        for (var header : responseHeaders.entrySet()) {
            headers.put(header.getKey().toLowerCase(java.util.Locale.ROOT), header.getValue());
        }
        String text = written.toString(java.nio.charset.StandardCharsets.UTF_8);
        return __YonAnswer.json(text).headers(headers).status(responseCode);
    }
}

final class __YonAnswer {
    private int status;
    private final java.util.Map<String, java.util.List<String>> headers = new java.util.LinkedHashMap<>();
    private String body;

    private __YonAnswer(int status, String body) {
        this.status = status;
        this.body = body;
    }

    public static __YonAnswer json(String body) {
        __YonAnswer response = new __YonAnswer(200, body);
        response.headers.put("content-type", java.util.List.of("application/json"));
        return response;
    }

    public static __YonAnswer empty(int status) {
        return new __YonAnswer(status, "");
    }

    public __YonAnswer status(int status) {
        this.status = status;
        return this;
    }

    __YonAnswer headers(java.util.Map<String, java.util.List<String>> replacement) {
        headers.clear();
        headers.putAll(replacement);
        return this;
    }

    public __YonAnswer header(String name, String value) {
        headers.computeIfAbsent(name, key -> new java.util.ArrayList<>()).add(value);
        return this;
    }

    void writeTo(com.sun.net.httpserver.HttpExchange exchange) throws java.io.IOException {
        for (var header : headers.entrySet()) {
            for (String value : header.getValue()) exchange.getResponseHeaders().add(header.getKey(), value);
        }
        byte[] payload = body.getBytes(java.nio.charset.StandardCharsets.UTF_8);
        exchange.sendResponseHeaders(status, payload.length == 0 ? -1 : payload.length);
        exchange.getResponseBody().write(payload);
    }

    String envelope() {
        java.util.Map<String, Object> envelope = new java.util.LinkedHashMap<>();
        envelope.put("status", status);
        envelope.put("headers", headers);
        envelope.put("body", body);
        return Json.write(envelope);
    }
}

final class Yon {
    private static final int MAX_CAUSE_DEPTH = 8;
    private static final int CHECK_REJECTED_STATUS = 422;
    static final int MAX_RELAY_STDOUT_BYTES = 16 * 1024 * 1024;
    static final int MAX_RELAY_STDERR_BYTES = 64 * 1024;
    private static final long DEFAULT_DEADLINE_MILLISECONDS = 30_000;
    private static final long NANOSECONDS_PER_MILLISECOND = 1_000_000;
    private static final String FAILURE_MESSAGE = "Component invocation failed.";
    private static long startNanos;
    private static java.util.Map<String, String> relayEnvironment;
    private static Long deadlineNanos;
    private static String requestId = "unknown";
    private static YonExchange exchange;

    static YonExchange exchange(__YonFrame frame) {
        if (exchange == null) exchange = new YonExchange(frame);
        return exchange;
    }

    static final class RelayOutput {
        final byte[] bytes;
        final boolean overflow;
        RelayOutput(byte[] bytes, boolean overflow) {
            this.bytes = bytes;
            this.overflow = overflow;
        }
    }

    static RelayOutput drain(java.io.InputStream pipe, int limit) {
        var kept = new java.io.ByteArrayOutputStream(Math.min(limit, 8192));
        var chunk = new byte[8192];
        boolean overflow = false;
        try (pipe) {
            while (true) {
                int count = pipe.read(chunk);
                if (count < 0) break;
                int remaining = Math.max(0, limit - kept.size());
                kept.write(chunk, 0, Math.min(count, remaining));
                overflow |= count > remaining;
            }
        } catch (java.io.IOException ignored) {
            overflow = true;
        }
        return new RelayOutput(kept.toByteArray(), overflow);
    }

    static void killTree(Process child) {
        child.descendants().forEach(handle -> handle.destroyForcibly());
        child.destroyForcibly();
    }

    private static void sendOutgoingThroughGuard() {
        String guard = System.getenv("YON_OUTGOING_PROXY");
        if (guard == null || guard.isEmpty()) return;
        java.net.URI address;
        try { address = java.net.URI.create(guard); } catch (IllegalArgumentException invalid) { return; }
        if (address.getHost() == null || address.getPort() < 0) return;
        for (String scheme : new String[] {"http", "https"}) {
            System.setProperty(scheme + ".proxyHost", address.getHost());
            System.setProperty(scheme + ".proxyPort", String.valueOf(address.getPort()));
        }
        System.setProperty("http.nonProxyHosts", "");
    }

    private static void captureStart() {
        if (relayEnvironment != null) return;
        long uptimeMilliseconds = java.lang.management.ManagementFactory.getRuntimeMXBean().getUptime();
        startNanos = System.nanoTime() - uptimeMilliseconds * NANOSECONDS_PER_MILLISECOND;
        relayEnvironment = relayEnvironmentFrom(System.getenv());
    }

    private static java.util.Map<String, String> relayEnvironmentFrom(java.util.Map<String, String> environment) {
        var kept = new java.util.HashMap<String, String>();
        for (var entry : environment.entrySet()) {
            String upper = entry.getKey().toUpperCase(java.util.Locale.ROOT);
            if (upper.equals("YON_APPLICATION_JSON") || upper.startsWith("YON_RESOURCE_")) continue;
            kept.put(entry.getKey(), entry.getValue());
        }
        return java.util.Collections.unmodifiableMap(kept);
    }

    private static void captureInvocation(__YonFrame request) {
        if (deadlineNanos != null) return;
        if (request.raw().get("request_id") instanceof String id) requestId = id;
        deadlineNanos = startNanos + invocationDeadlineMilliseconds(request) * NANOSECONDS_PER_MILLISECOND;
    }

    private static long invocationDeadlineMilliseconds(__YonFrame request) {
        if (request.raw().get("deadline_ms") instanceof Number milliseconds) return milliseconds.longValue();
        return DEFAULT_DEADLINE_MILLISECONDS;
    }

    static void renewDeadline() {
        deadlineNanos = System.nanoTime() + DEFAULT_DEADLINE_MILLISECONDS * NANOSECONDS_PER_MILLISECOND;
    }

    static long timeLeftNanos() {
        if (deadlineNanos == null) return 0;
        return deadlineNanos - System.nanoTime();
    }

    static void relay(java.util.List<String> command, com.sun.net.httpserver.HttpExchange exchange) throws java.io.IOException {
        __YonAnswer answer = relayAnswer(command, exchange);
        answer.writeTo(exchange);
    }

    private static __YonAnswer relayAnswer(java.util.List<String> command, com.sun.net.httpserver.HttpExchange exchange) throws java.io.IOException {
        if (command.isEmpty()) return relayFailed("start");
        if (timeLeftNanos() <= 0) return relayFailed("timeout");
        byte[] payload;
        try {
            payload = Json.write(relayMessage(exchange)).getBytes(java.nio.charset.StandardCharsets.UTF_8);
        } catch (IllegalArgumentException invalid) {
            return relayFailed("protocol");
        }
        Process child;
        try {
            child = startChild(command);
        } catch (java.io.IOException | RuntimeException error) {
            return relayFailed("start");
        }
        var stdoutTask = java.util.concurrent.CompletableFuture.supplyAsync(
            () -> drain(child.getInputStream(), MAX_RELAY_STDOUT_BYTES), Yon::startDaemon);
        var stderrTask = java.util.concurrent.CompletableFuture.supplyAsync(
            () -> drain(child.getErrorStream(), MAX_RELAY_STDERR_BYTES), Yon::startDaemon);
        startDaemon(() -> sendRequest(child, payload));
        return awaitChild(child, stdoutTask, stderrTask);
    }

    private static java.util.Map<String, Object> relayMessage(com.sun.net.httpserver.HttpExchange exchange) throws java.io.IOException {
        var headers = new java.util.LinkedHashMap<String, Object>();
        for (var header : exchange.getRequestHeaders().entrySet()) {
            headers.put(header.getKey().toLowerCase(java.util.Locale.ROOT), header.getValue());
        }
        String body = new String(exchange.getRequestBody().readAllBytes(), java.nio.charset.StandardCharsets.UTF_8);
        var message = new java.util.LinkedHashMap<String, Object>();
        message.put("route", exchange.getRequestURI().toString());
        message.put("method", exchange.getRequestMethod());
        message.put("headers", headers);
        message.put("body", java.util.Map.of("encoding", "utf8", "data", body));
        return message;
    }

    private static Process startChild(java.util.List<String> command) throws java.io.IOException {
        var builder = new ProcessBuilder(command);
        builder.environment().clear();
        builder.environment().putAll(relayEnvironment);
        return builder.start();
    }

    private static void startDaemon(Runnable task) {
        Thread thread = new Thread(task);
        thread.setDaemon(true);
        thread.start();
    }

    private static void sendRequest(Process child, byte[] payload) {
        try (var stdin = child.getOutputStream()) {
            stdin.write(payload);
        } catch (java.io.IOException ignored) {
        }
    }

    private static __YonAnswer awaitChild(
        Process child,
        java.util.concurrent.CompletableFuture<RelayOutput> stdoutTask,
        java.util.concurrent.CompletableFuture<RelayOutput> stderrTask
    ) {
        var nanoseconds = java.util.concurrent.TimeUnit.NANOSECONDS;
        try {
            if (!child.waitFor(timeLeftNanos(), nanoseconds)) return stopChild(child);
            RelayOutput stdout = stdoutTask.get(timeLeftNanos(), nanoseconds);
            RelayOutput stderr = stderrTask.get(timeLeftNanos(), nanoseconds);
            return relayResult(child.exitValue(), stdout, stderr);
        } catch (java.util.concurrent.TimeoutException unsettled) {
            return stopChild(child);
        } catch (InterruptedException interrupted) {
            Thread.currentThread().interrupt();
            return stopChild(child);
        } catch (java.util.concurrent.ExecutionException error) {
            return relayFailed("overflow");
        }
    }

    private static __YonAnswer stopChild(Process child) {
        killTree(child);
        closePipe(child.getInputStream());
        closePipe(child.getErrorStream());
        return relayFailed("timeout");
    }

    private static void closePipe(java.io.InputStream pipe) {
        try {
            pipe.close();
        } catch (java.io.IOException ignored) {
        }
    }

    private static __YonAnswer relayResult(int exitCode, RelayOutput stdout, RelayOutput stderr) {
        if (stdout.overflow || stderr.overflow) return relayFailed("overflow");
        if (exitCode != 0) return relayFailed("exit");
        String output = new String(stdout.bytes, java.nio.charset.StandardCharsets.UTF_8);
        Object parsed;
        try {
            parsed = Json.parse(output);
        } catch (IllegalArgumentException invalid) {
            return relayFailed("protocol");
        }
        if (!(parsed instanceof java.util.Map<?, ?> envelope)) return relayFailed("protocol");
        return responseFromEnvelope(envelope);
    }

    private static __YonAnswer responseFromEnvelope(java.util.Map<?, ?> envelope) {
        Object body = envelope.get("body");
        __YonAnswer response = __YonAnswer.json(body == null ? "" : String.valueOf(body));
        Object code = envelope.get("status");
        if (code instanceof Number number) response = response.status(number.intValue());
        if (!(envelope.get("headers") instanceof java.util.Map<?, ?> written) || written.isEmpty()) return response;
        response = response.headers(java.util.Map.of());
        for (java.util.Map.Entry<?, ?> entry : written.entrySet()) {
            Object values = entry.getValue();
            java.util.List<?> list = values instanceof java.util.List<?> items ? items : java.util.List.of(values);
            for (Object value : list) {
                response = response.header(String.valueOf(entry.getKey()), String.valueOf(value));
            }
        }
        return response;
    }

    private static __YonAnswer relayFailed(String category) {
        var event = new java.util.LinkedHashMap<String, Object>();
        event.put("event", "handler.relay_failed");
        event.put("request_id", requestId);
        event.put("category", category);
        System.err.print(Json.write(event) + "\n");
        System.err.flush();
        return __YonAnswer.json(Json.write(java.util.Map.of("error", FAILURE_MESSAGE))).status(502);
    }

    public static void main(String[] args) throws java.io.IOException {
        sendOutgoingThroughGuard();
        captureStart();
        String raw = YonSocket.framed() ? YonSocket.readFrame() : read(System.in);
        if (raw == null) return;
        __YonFrame request = new __YonFrame(Json.parse(raw));
        captureInvocation(request);
        String phase = switch (String.valueOf(request.raw().get("operation"))) {
            case "middleware.before" -> "before";
            case "middleware.after" -> "after";
            case "event.deliver" -> "__yonDeliver";
            default -> null;
        };
        String entryName = phase == null ? request.method() : phase;
        int missingStatus = phase == null ? 405 : "__yonDeliver".equals(phase) ? 404 : 204;
        __YonAnswer response;
        try {
            java.lang.reflect.Method method = entryMethod(entryName);
            method.setAccessible(true);
            Object result = method.invoke(null, entryArguments(method, request));
            if (result instanceof java.util.stream.Stream<?> || result instanceof java.util.Iterator<?>) {
                YonStream.send(String.valueOf(request.raw().getOrDefault("request_id", "")), result);
                return;
            }
            if (exchange != null && exchange.answered()) response = exchange.answer();
            else if (result instanceof __YonAnswer explicit) response = explicit;
            else if (phase != null) response = __YonAnswer.empty(204);
            else response = __YonAnswer.json(Json.stringify(result));
        } catch (NoSuchMethodException absent) {
            response = __YonAnswer.empty(missingStatus);
        } catch (java.lang.reflect.InvocationTargetException failed) {
            java.util.Map<?, ?> rejection = rejection(failed.getCause());
            if (rejection == null) {
                System.exit(1);
                return;
            }
            System.out.print(rejectionEnvelope(rejection));
            return;
        } catch (java.lang.ReflectiveOperationException failed) {
            System.exit(1);
            return;
        }
        System.out.print(response.envelope());
    }

    private static java.util.Map<?, ?> rejection(Throwable failure) {
        Throwable cause = failure;
        for (int depth = 0; cause != null && depth < MAX_CAUSE_DEPTH; depth++) {
            if (cause instanceof java.util.function.Supplier<?> supplier && supplier.get() instanceof java.util.Map<?, ?> found && found.get("violations") instanceof java.util.List<?>) return found;
            cause = cause.getCause();
        }
        return null;
    }

    private static String rejectionEnvelope(java.util.Map<?, ?> rejection) {
        var envelope = new java.util.LinkedHashMap<String, Object>();
        envelope.put("status", CHECK_REJECTED_STATUS);
        envelope.put("violations", rejection.get("violations"));
        return Json.write(envelope);
    }

    private static java.lang.reflect.Method entryMethod(String name) throws NoSuchMethodException {
        for (java.lang.reflect.Method method : __YON_HANDLER__.class.getDeclaredMethods()) {
            if (method.getName().equals(name) && method.getParameterCount() <= 1) return method;
        }
        throw new NoSuchMethodException(name);
    }

    private static Object[] entryArguments(java.lang.reflect.Method method, __YonFrame request) {
        if (method.getParameterCount() == 0) return new Object[0];
        if (method.getParameterTypes()[0] == __YonFrame.class) return new Object[] {request};
        return new Object[] {exchange(request)};
    }

    private static String read(java.io.InputStream stream) throws java.io.IOException {
        return new String(stream.readAllBytes(), java.nio.charset.StandardCharsets.UTF_8);
    }
}
