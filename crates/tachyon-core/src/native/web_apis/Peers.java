final class Peers {
    private static final String API = "peers";

    boolean supported() {
        return TacWebHost.flag(API, "supported");
    }

    String open(String username, String credential) {
        return String.valueOf(TacWebHost.call(API, "open", username, credential));
    }

    void offer(String connection) {
        TacWebHost.call(API, "offer", connection);
    }

    void answer(String connection, String offerSdp) {
        TacWebHost.call(API, "answer", connection, offerSdp);
    }

    void accept(String connection, String answerSdp) {
        TacWebHost.call(API, "accept", connection, answerSdp);
    }

    void addCandidate(String connection, String candidateJson) {
        TacWebHost.call(API, "addCandidate", connection, candidateJson);
    }

    void restart(String connection) {
        TacWebHost.call(API, "restart", connection);
    }

    String openChannel(String connection, String label, boolean ordered) {
        return String.valueOf(TacWebHost.call(API, "openChannel", connection, label, ordered));
    }

    boolean send(String connection, String channel, String data) {
        return Boolean.TRUE.equals(TacWebHost.call(API, "send", connection, channel, data));
    }

    void closeChannel(String connection, String channel) {
        TacWebHost.call(API, "closeChannel", connection, channel);
    }

    void close(String connection) {
        TacWebHost.call(API, "close", connection);
    }
}
