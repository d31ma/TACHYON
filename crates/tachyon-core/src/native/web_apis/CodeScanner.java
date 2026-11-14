final class CodeScanner {
    private static final String API = "codeScanner";

    boolean supported() {
        return TacWebHost.flag(API, "supported");
    }

    void scan() {
        TacWebHost.call(API, "scan");
    }

    void cancel() {
        TacWebHost.call(API, "cancel");
    }
}
