final class StorageSpace {
    private static final String API = "storageSpace";

    void estimate() {
        TacWebHost.call(API, "estimate");
    }

    void persist() {
        TacWebHost.call(API, "persist");
    }
}
