final class TacWebHost {
    private static final String NO_PAGE = "This browser API needs the page, and this service runs without one.";
    private static final String WRONG_ANSWER = "The page gave a browser API answer of the wrong type.";

    private TacWebHost() {
    }

    static Object call(String api, String call, Object... arguments) {
        java.util.Map<String, Object> request = new java.util.LinkedHashMap<>();
        request.put("api", api);
        request.put("call", call);
        request.put("args", java.util.Arrays.asList(arguments));
        Object answer = TacJsonReader.read(TacHost.web(TacJsonWriter.write(request)));
        if (!(answer instanceof java.util.Map<?, ?> entries)) {
            throw new IllegalStateException(NO_PAGE);
        }
        Object error = entries.get("error");
        if (error != null) {
            throw new IllegalStateException(error.toString());
        }
        return entries.get("value");
    }

    static String text(String api, String call) {
        if (!(call(api, call) instanceof String text)) {
            throw new IllegalStateException(WRONG_ANSWER);
        }
        return text;
    }

    static boolean flag(String api, String call) {
        if (!(call(api, call) instanceof Boolean flag)) {
            throw new IllegalStateException(WRONG_ANSWER);
        }
        return flag;
    }

    static double number(String api, String call) {
        return numeric(call(api, call)).doubleValue();
    }

    static long whole(Object value) {
        return numeric(value).longValue();
    }

    static java.util.List<?> list(Object value) {
        if (!(value instanceof java.util.List<?> values)) {
            throw new IllegalStateException(WRONG_ANSWER);
        }
        return values;
    }

    private static Number numeric(Object value) {
        if (!(value instanceof Number number)) {
            throw new IllegalStateException(WRONG_ANSWER);
        }
        return number;
    }
}
