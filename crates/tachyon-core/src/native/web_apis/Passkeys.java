final class Passkeys {
    private static final String API = "passkeys";

    boolean supported() {
        return TacWebHost.flag(API, "supported");
    }

    void check() {
        TacWebHost.call(API, "check");
    }

    void create(String optionsJson) {
        TacWebHost.call(API, "create", optionsJson);
    }

    void get(String optionsJson) {
        TacWebHost.call(API, "get", optionsJson);
    }

    void offer(String optionsJson) {
        TacWebHost.call(API, "offer", optionsJson);
    }

    void cancel() {
        TacWebHost.call(API, "cancel");
    }
}
