final class TacSocket {
    private static final int MOST_BYTES = 65536;
    private static final java.util.Set<String> CODES = java.util.Set.of("HTTP_MISSING", "HTTP_REQUEST", "CANCELLED");
    private static final String TOO_LARGE = "A socket message is JSON of at most 64 KiB.";
    private static final String NOT_SENT = "The socket message was not sent.";

    private TacSocket() {
    }

    static void send(String client, Object message) {
        String text = encoded(message);
        if (text == null || text.getBytes(java.nio.charset.StandardCharsets.UTF_8).length > MOST_BYTES) {
            throw new Failure("HTTP_BODY", TOO_LARGE);
        }
        String request = "{\"client\":" + TacJsonWriter.quote(client) + ",\"message\":" + text + "}";
        Object answer = read(TacHttpTickets.socket(request));
        if (answer instanceof java.util.Map<?, ?> entries && entries.get("error") == null && entries.containsKey("value")) {
            return;
        }
        throw new Failure(code(answer), NOT_SENT);
    }

    private static String encoded(Object message) {
        try {
            return TacJsonWriter.write(message);
        } catch (RuntimeException problem) {
            return null;
        }
    }

    private static Object read(String answer) {
        try {
            return TacJsonReader.read(answer);
        } catch (RuntimeException problem) {
            return null;
        }
    }

    private static String code(Object answer) {
        Object error = answer instanceof java.util.Map<?, ?> entries ? entries.get("error") : null;
        Object code = error instanceof java.util.Map<?, ?> named ? named.get("code") : null;
        return code instanceof String text && CODES.contains(text) ? text : "HTTP_NETWORK";
    }

    static final class Failure extends TacFailure {
        Failure(String code, String message) {
            super(code, code + ": " + message);
        }
    }
}
