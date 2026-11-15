final class Passkeys {
    private static final String API = "passkeys";

    boolean supported() {
        return TacWebHost.flag(API, "supported");
    }

    void check() {
        TacWebHost.call(API, "check");
    }

    void create(String optionsJson, String prfKey) {
        TacWebHost.call(API, "create", optionsJson, prfKey);
    }

    void get(String optionsJson, String prfKey) {
        TacWebHost.call(API, "get", optionsJson, prfKey);
    }

    void offer(String optionsJson, String prfKey) {
        TacWebHost.call(API, "offer", optionsJson, prfKey);
    }

    void cancel() {
        TacWebHost.call(API, "cancel");
    }
}
