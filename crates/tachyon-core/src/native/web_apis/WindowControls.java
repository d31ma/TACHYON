final class WindowControls {
    private static final String API = "windowControls";

    boolean granted(String control) {
        return Boolean.TRUE.equals(TacWebHost.call(API, "granted", control));
    }

    void minimize() {
        TacWebHost.call(API, "minimize");
    }

    void maximize() {
        TacWebHost.call(API, "maximize");
    }

    void fullscreen() {
        TacWebHost.call(API, "fullscreen");
    }

    void close() {
        TacWebHost.call(API, "close");
    }

    void resize(int width, int height) {
        TacWebHost.call(API, "resize", width, height);
    }

    void setTitle(String text) {
        TacWebHost.call(API, "setTitle", text);
    }
}
