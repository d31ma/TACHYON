final class TacEvent {
    final String name;
    final String json;
    final boolean retain;

    TacEvent(String name, String json, boolean retain) {
        this.name = name;
        this.json = json;
        this.retain = retain;
    }
}
