final class TacActivation {
    final String route;
    final String service;
    final String owner;
    final TacNode node;
    final java.util.Map<String, Object> members;
    final java.util.ArrayDeque<TacEvent> queue = new java.util.ArrayDeque<>();
    boolean disposed;
    final TacPreferenceScope preferences = new TacPreferenceScope();

    TacActivation(String route, String service, String owner) {
        this.route = route;
        this.service = service;
        this.owner = owner;
        this.node = created(service);
        this.members = node.members;
    }

    private TacNode created(String service) {
        try {
            return TacNeeds.attach(service, this);
        } catch (RuntimeException failed) {
            preferences.dispose();
            throw failed;
        }
    }

    boolean belongsTo(String route, String service) {
        return this.route.equals(route) && this.service.equals(service);
    }

    void start() {
        TacNeeds.start(node);
    }

    void abandon() {
        preferences.dispose();
        TacNeeds.release(node);
    }

    void dispose() {
        disposed = true;
        preferences.dispose();
        queue.clear();
        TacNeeds.release(node);
    }
}
