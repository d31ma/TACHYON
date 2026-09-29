final class Notifications {
    private static final String API = "notifications";

    String permission() {
        return TacWebHost.text(API, "permission");
    }

    void requestPermission() {
        TacWebHost.call(API, "requestPermission");
    }

    void show(String title, String body) {
        TacWebHost.call(API, "show", title, body);
    }
}
