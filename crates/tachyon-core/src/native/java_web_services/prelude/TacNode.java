final class TacNode {
    final String name;
    final TacPreferenceScope preferences;
    final java.util.List<TacNode> needs = new java.util.ArrayList<>();
    java.util.Map<String, Object> members;
    Object service;
    int holders;
    boolean started;
    boolean failed;

    TacNode(String name, TacPreferenceScope preferences) {
        this.name = name;
        this.preferences = preferences;
    }

    void runHooks(boolean destroy) {
        for (Object member : new java.util.TreeMap<>(members).values()) {
            if (member instanceof TacHook hook && hook.destroy == destroy) {
                hook.invoke.run();
            }
        }
    }
}
