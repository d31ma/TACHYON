final class Clipboard {
    private static final String API = "clipboard";

    void writeText(String text) {
        TacWebHost.call(API, "writeText", text);
    }

    void readText() {
        TacWebHost.call(API, "readText");
    }
}
