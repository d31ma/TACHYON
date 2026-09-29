final class TacBridge {
    private static final int START_UP_REQUEST_LIMIT = 1048576;
    private static final int MEMBER_REQUEST_LIMIT = 65536;
    private static final int MOST_ACTIVATIONS = 256;
    private static final int DEEPEST = 64;
    private static final int OWNER_LENGTH = 36;
    private static final String NOTHING = "{\"value\":null}";
    private static final String MISSING_REQUIRED = "{\"error\":\"Native service is missing a required property or it has the wrong type.\",\"code\":\"TY_NATIVE_REQUIRED\"}";
    private static final String UNKNOWN_SERVICE = "{\"error\":\"Unknown native service.\",\"code\":\"TY_NATIVE_SERVICE\"}";
    private static final java.util.Map<String, TacActivation> ACTIVATIONS = new java.util.LinkedHashMap<>();
    private static boolean handling;

    private TacBridge() {
    }

    static String handle(String request) {
        if (handling) {
            return error("Native owner is busy.");
        }
        handling = true;
        try {
            return answer(request);
        } catch (RuntimeException problem) {
            String message = problem.getMessage();
            return error(message == null ? "Java service operation failed." : message);
        } finally {
            TacEvents.drain();
            handling = false;
        }
    }

    private static String answer(String request) {
        int size = request.getBytes(java.nio.charset.StandardCharsets.UTF_8).length;
        require(size <= START_UP_REQUEST_LIMIT, tooLarge("request", size, START_UP_REQUEST_LIMIT));
        require(bounded(request), "Invalid companion request.");
        java.util.Map<String, Object> parsed = entries(TacJsonReader.read(request));
        require(parsed != null, "Invalid companion request.");
        String operation = text(parsed.get("op"));
        String name = text(parsed.get("name"));
        String subject = "request to " + (name.isEmpty() ? operation : name);
        require(operation.equals("init") || size <= MEMBER_REQUEST_LIMIT, tooLarge(subject, size, MEMBER_REQUEST_LIMIT));
        if (operation.equals("host-dispose")) {
            disposeAll();
            return NOTHING;
        }
        String route = text(parsed.get("route"));
        String owner = text(parsed.get("owner"));
        String service = text(parsed.get("service"));
        require(isOwnerId(owner), "Missing native owner identity.");
        require(!service.isEmpty(), "Missing companion name.");
        if (operation.equals("init")) {
            String refusal = activate(route, service, owner, entries(parsed.get("props")));
            if (refusal != null) {
                return refusal;
            }
        }
        TacActivation activation = ACTIVATIONS.get(owner);
        require(activation != null && activation.belongsTo(route, service), "Stale native owner.");
        if (operation.equals("dispose")) {
            dispose(owner);
            return NOTHING;
        }
        TacEvents.runFor(activation);
        if (operation.equals("init")) {
            return describe(activation.members);
        }
        return use(activation.members, operation, name, parsed);
    }

    private static String activate(String route, String service, String owner, java.util.Map<String, Object> props) {
        TacActivation existing = ACTIVATIONS.get(owner);
        if (existing != null && existing.belongsTo(route, service)) {
            return null;
        }
        if (existing != null) {
            dispose(owner);
        }
        java.util.Map<String, Object> members = TacServiceTable.members(service);
        if (members == null) {
            return UNKNOWN_SERVICE;
        }
        require(ACTIVATIONS.size() < MOST_ACTIVATIONS, "Too many native activations.");
        TacActivation activation = new TacActivation(route, service, owner, members);
        try {
            if (!assign(activation, props)) {
                return MISSING_REQUIRED;
            }
        } catch (RuntimeException problem) {
            return MISSING_REQUIRED;
        }
        ACTIVATIONS.put(owner, activation);
        TacEvents.runFor(activation);
        try {
            activation.runHooks(false);
        } catch (RuntimeException problem) {
            dispose(owner);
            throw problem;
        }
        return null;
    }

    private static boolean assign(TacActivation activation, java.util.Map<String, Object> props) {
        java.util.List<String> required = java.util.Arrays.asList(TacServiceTable.required(activation.service));
        boolean everyField = required.contains("*");
        for (String name : required) {
            if (!name.equals("*") && (props == null || props.get(name) == null)) {
                return false;
            }
        }
        if (props == null) {
            return true;
        }
        for (java.util.Map.Entry<String, Object> property : props.entrySet()) {
            Object member = activation.members.get(property.getKey());
            boolean wanted = everyField || required.contains(property.getKey());
            if (wanted && member instanceof TacField field && field.write != null) {
                field.write.accept(property.getValue());
            }
        }
        return true;
    }

    private static String use(java.util.Map<String, Object> members, String operation, String name, java.util.Map<String, Object> parsed) {
        Object member = members.get(name);
        if (operation.equals("event")) {
            return receive(member, name, parsed.get("value"));
        }
        if (member instanceof TacField field && operation.equals("get")) {
            return "{\"value\":" + TacJsonWriter.write(field.read.get()) + "}";
        }
        if (member instanceof TacField field && operation.equals("set")) {
            if (field.write == null) {
                return error("Companion field is read-only: " + name);
            }
            field.write.accept(parsed.get("value"));
            return NOTHING;
        }
        if (member instanceof TacMethod method && operation.equals("call")) {
            Object given = parsed.get("args");
            java.util.List<Object> arguments = given instanceof java.util.List<?> list
                    ? new java.util.ArrayList<>(list)
                    : new java.util.ArrayList<>();
            return "{\"value\":" + TacJsonWriter.write(method.invoke.apply(arguments)) + "}";
        }
        if (member instanceof TacField || member instanceof TacMethod) {
            return error("Companion member does not support " + operation + ": " + name);
        }
        return error("Unknown companion member: " + name);
    }

    private static String receive(Object member, String name, Object value) {
        if (!(member instanceof TacListener listener)) {
            return error("Unknown companion listener: " + name);
        }
        Object result = listener.invoke.apply(value);
        if (listener.republishes()) {
            TacEvents.republish(result);
        }
        return NOTHING;
    }

    private static String describe(java.util.Map<String, Object> members) {
        java.util.List<String> fields = new java.util.ArrayList<>();
        java.util.List<String> methods = new java.util.ArrayList<>();
        java.util.List<Object> listeners = new java.util.ArrayList<>();
        for (java.util.Map.Entry<String, Object> member : members.entrySet()) {
            if (member.getValue() instanceof TacField) {
                fields.add(member.getKey());
            } else if (member.getValue() instanceof TacMethod) {
                methods.add(member.getKey());
            } else if (member.getValue() instanceof TacListener listener) {
                listeners.add(listener.described(member.getKey()));
            }
        }
        return "{\"value\":{\"fields\":" + TacJsonWriter.write(fields) + ",\"methods\":"
                + TacJsonWriter.write(methods) + ",\"listeners\":" + TacJsonWriter.write(listeners) + "}}";
    }

    private static void dispose(String owner) {
        TacActivation activation = ACTIVATIONS.remove(owner);
        if (activation != null) {
            activation.dispose();
        }
    }

    private static void disposeAll() {
        for (String owner : new java.util.ArrayList<>(ACTIVATIONS.keySet())) {
            dispose(owner);
        }
    }

    private static boolean bounded(String raw) {
        int depth = 0;
        boolean quoted = false;
        boolean escaped = false;
        for (int index = 0; index < raw.length(); index++) {
            char character = raw.charAt(index);
            if (quoted) {
                if (escaped) {
                    escaped = false;
                } else if (character == '\\') {
                    escaped = true;
                } else if (character == '"') {
                    quoted = false;
                }
                continue;
            }
            if (character == '"') {
                quoted = true;
            } else if (character == '{' || character == '[') {
                depth++;
            } else if (character == '}' || character == ']') {
                depth--;
            }
            if (depth > DEEPEST || depth < 0) {
                return false;
            }
        }
        return depth == 0 && !quoted;
    }

    private static boolean isOwnerId(String owner) {
        if (owner.length() != OWNER_LENGTH) {
            return false;
        }
        String lower = owner.toLowerCase();
        for (int index = 0; index < lower.length(); index++) {
            boolean dash = index == 8 || index == 13 || index == 18 || index == 23;
            char character = lower.charAt(index);
            boolean fits = dash ? character == '-' : "0123456789abcdef".indexOf(character) >= 0;
            if (!fits) {
                return false;
            }
        }
        return true;
    }

    @SuppressWarnings("unchecked")
    private static java.util.Map<String, Object> entries(Object value) {
        return value instanceof java.util.Map<?, ?> ? (java.util.Map<String, Object>) value : null;
    }

    private static String text(Object value) {
        return value == null ? "" : value.toString();
    }

    private static String tooLarge(String subject, int size, int limit) {
        return "The " + subject + " is " + size + " bytes. The limit is " + limit + " bytes. Pass less data to the service.";
    }

    private static void require(boolean holds, String problem) {
        if (!holds) {
            throw new IllegalArgumentException(problem);
        }
    }

    private static String error(String message) {
        return "{\"error\":" + TacJsonWriter.quote(message) + "}";
    }
}
