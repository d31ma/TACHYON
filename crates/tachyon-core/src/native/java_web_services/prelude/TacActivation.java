final class TacActivation {
    final String route;
    final String service;
    final String owner;
    final java.util.Map<String, Object> members;
    final java.util.ArrayDeque<TacEvent> queue = new java.util.ArrayDeque<>();
    boolean disposed;

    TacActivation(String route, String service, String owner, java.util.Map<String, Object> members) {
        this.route = route;
        this.service = service;
        this.owner = owner;
        this.members = members;
    }

    boolean belongsTo(String route, String service) {
        return this.route.equals(route) && this.service.equals(service);
    }

    void runHooks(boolean destroy) {
        for (Object member : new java.util.TreeMap<>(members).values()) {
            if (member instanceof TacHook hook && hook.destroy == destroy) {
                hook.invoke.run();
            }
        }
    }

    void dispose() {
        disposed = true;
        queue.clear();
        runHooks(true);
    }
}
