@Service
class EventService {
  private static final String COLLECTION = "hd-events";
  private static final String KIND = "kind";
  private static final String COUNT = "count";
  private static final int PAGE_SIZE = 2;
  private static final String TOPIC = "app.render";
  private static final String GROUP = "renderers";

  @Needs
  DataSource dataSource;

  java.util.Map<String, Object> report() {
    dataSource.createCollection(COLLECTION);
    var id = (String) ((java.util.Map<?, ?>) dataSource.insert(COLLECTION, deploys())).get("id");
    dataSource.insert(COLLECTION, deploys());
    dataSource.insert(COLLECTION, deploys());
    var found = (java.util.Map<?, ?>) dataSource.findById(COLLECTION, id);
    var patched = (java.util.Map<?, ?>) dataSource.patch(COLLECTION, id, change());
    var report = countPages();
    report.put("found", found.get(KIND));
    report.put("patched", patched.get(COUNT));
    report.put("job", runJob());
    dataSource.deleteById(COLLECTION, id);
    report.put("gone", dataSource.findById(COLLECTION, id) == null);
    report.put("dropped", dataSource.dropCollection(COLLECTION));
    return report;
  }

  private java.util.Map<String, Object> deploys() {
    return java.util.Map.of(KIND, "deploy");
  }

  private java.util.Map<String, Object> change() {
    return java.util.Map.of(COUNT, 7);
  }

  private java.util.Map<String, Object> countPages() {
    int pages = 0;
    int seen = 0;
    String cursor = null;
    boolean more = true;
    while (more) {
      var window = (java.util.Map<?, ?>) dataSource.scroll(COLLECTION, deploys(), cursor, PAGE_SIZE);
      pages += 1;
      seen += ((java.util.List<?>) window.get("content")).size();
      cursor = (String) window.get("nextCursor");
      more = cursor != null;
    }
    var counts = new java.util.LinkedHashMap<String, Object>();
    counts.put("pages", pages);
    counts.put("seen", seen);
    return counts;
  }

  private Object runJob() {
    dataSource.publish(TOPIC, java.util.Map.of("app", "web"), "render:web");
    var delivery = (java.util.Map<?, ?>) ((java.util.List<?>) dataSource.claim(TOPIC, GROUP, 1, 30000)).get(0);
    dataSource.ack(TOPIC, GROUP, delivery);
    return delivery.get("payload");
  }
}
