final class Preferences {
    private static final String API = "preferences";

    String scheme() {
        return TacWebHost.text(API, "scheme");
    }

    String motion() {
        return TacWebHost.text(API, "motion");
    }

    String language() {
        return TacWebHost.text(API, "language");
    }
}
