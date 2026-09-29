final class TacListener {
    private static final String KEY = "key";
    private static final String RETURNS = "returns";
    private static final String IN_BACKGROUND = "asynchronous";
    final java.util.Map<String, Object> description;
    final java.util.function.Function<Object, Object> invoke;

    @SuppressWarnings("unchecked")
    TacListener(String description, java.util.function.Function<Object, Object> invoke) {
        this.description = (java.util.Map<String, Object>) TacJsonReader.read(description);
        this.invoke = invoke;
    }

    boolean inBackground() {
        return Boolean.TRUE.equals(description.get(IN_BACKGROUND));
    }

    boolean republishes() {
        return !inBackground() && description.get(RETURNS) != null;
    }

    java.util.Map<String, Object> described(String key) {
        java.util.Map<String, Object> described = new java.util.LinkedHashMap<>(description);
        described.put(KEY, key);
        return described;
    }
}
