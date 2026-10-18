final class TacSocket {
    private TacSocket() {
    }

    static void send(String client, Object message) {
        throw new IllegalStateException("HTTP_MISSING: only a browser page holds the " + client + " socket.");
    }
}
