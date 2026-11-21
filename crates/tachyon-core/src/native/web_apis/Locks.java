final class Locks {
    private static final String API = "locks";

    void request(String name) {
        TacWebHost.call(API, "request", name);
    }

    void release(String name) {
        TacWebHost.call(API, "release", name);
    }
}
