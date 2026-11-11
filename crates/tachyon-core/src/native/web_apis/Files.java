final class Files {
    private static final String API = "files";

    void pick(String accept, boolean multiple) {
        TacWebHost.call(API, "pick", accept, multiple);
    }

    void read(String id, long offset, int count) {
        TacWebHost.call(API, "read", id, offset, count);
    }

    void release(String id) {
        TacWebHost.call(API, "release", id);
    }

    String upload(String id, String method, String url, String headers) {
        return String.valueOf(TacWebHost.call(API, "upload", id, method, url, headers));
    }

    void cancel(String upload) {
        TacWebHost.call(API, "cancel", upload);
    }
}
