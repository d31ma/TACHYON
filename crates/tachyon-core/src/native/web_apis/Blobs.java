final class Blobs {
    private static final String API = "blobs";

    void saveHeld(String fileId, String key) {
        TacWebHost.call(API, "saveHeld", fileId, key);
    }

    void download(String url, String key, String headers) {
        TacWebHost.call(API, "download", url, key, headers);
    }

    void list(String prefix) {
        TacWebHost.call(API, "list", prefix);
    }

    void read(String key, long offset, int count) {
        TacWebHost.call(API, "read", key, offset, count);
    }

    void remove(String key) {
        TacWebHost.call(API, "remove", key);
    }

    String url(String key) {
        return String.valueOf(TacWebHost.call(API, "url", key));
    }

    void open(String key) {
        TacWebHost.call(API, "open", key);
    }

    void close(String key) {
        TacWebHost.call(API, "close", key);
    }

    String upload(String key, String method, String url, String headers) {
        return String.valueOf(TacWebHost.call(API, "upload", key, method, url, headers));
    }

    void cancel(String upload) {
        TacWebHost.call(API, "cancel", upload);
    }
}
