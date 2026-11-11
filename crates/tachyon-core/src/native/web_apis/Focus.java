final class Focus {
    private static final String API = "focus";

    void move(String target) {
        TacWebHost.call(API, "move", target);
    }

    void trap(String container) {
        TacWebHost.call(API, "trap", container);
    }

    void release() {
        TacWebHost.call(API, "release");
    }
}
