final class TacStorage {
    private static final String NO_ANSWER = "Tachyon storage did not answer.";

    private TacStorage() {
    }

    static Object ask(java.util.Map<String, Object> request) {
        return value(answer(request));
    }

    static java.util.Map<?, ?> answer(java.util.Map<String, Object> request) {
        Object answer = TacJsonReader.read(TacHost.storage(TacJsonWriter.write(request)));
        if (!(answer instanceof java.util.Map<?, ?> entries)) {
            throw new IllegalStateException(NO_ANSWER);
        }
        return entries;
    }

    static Object value(java.util.Map<?, ?> entries) {
        Object error = entries.get("error");
        if (error != null) {
            throw new IllegalStateException(error.toString());
        }
        return entries.get("value");
    }
}
