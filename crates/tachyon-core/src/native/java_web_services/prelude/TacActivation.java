final class TacActivation {
    final String route;
    final String service;
    final java.util.Map<String, Object> members;

    TacActivation(String route, String service, java.util.Map<String, Object> members) {
        this.route = route;
        this.service = service;
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
}
