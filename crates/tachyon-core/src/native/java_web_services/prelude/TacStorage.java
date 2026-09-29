final class TacStorage {
    private static final String NO_ANSWER = "Tachyon storage did not answer.";

    private TacStorage() {
    }

    static Object ask(java.util.Map<String, Object> request) {
        Object answer = TacJsonReader.read(TacHost.storage(TacJsonWriter.write(request)));
        if (!(answer instanceof java.util.Map<?, ?> entries)) {
            throw new IllegalStateException(NO_ANSWER);
        }
        Object error = entries.get("error");
        if (error != null) {
            throw new IllegalStateException(error.toString());
        }
        return entries.get("value");
    }
}
