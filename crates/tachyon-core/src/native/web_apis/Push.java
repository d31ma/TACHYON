final class Push {
    private static final String API = "push";

    boolean supported() {
        return TacWebHost.flag(API, "supported");
    }

    void subscribe(String applicationServerKey) {
        TacWebHost.call(API, "subscribe", applicationServerKey);
    }

    void subscription() {
        TacWebHost.call(API, "subscription");
    }

    void unsubscribe() {
        TacWebHost.call(API, "unsubscribe");
    }
}
