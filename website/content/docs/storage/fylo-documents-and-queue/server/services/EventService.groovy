@Service
class EventService {
  private static final String COLLECTION = "hd-events"
  private static final Map DEPLOYS = [kind: "deploy"]
  private static final Map CHANGE = [count: 7]
  private static final int PAGE_SIZE = 2
  private static final String TOPIC = "app.render"
  private static final String GROUP = "renderers"

  @Needs
  DataSource dataSource

  Map<String, Object> report() {
    dataSource.createCollection(COLLECTION)
    String id = (String) ((Map) dataSource.insert(COLLECTION, DEPLOYS)).id
    dataSource.insert(COLLECTION, DEPLOYS)
    dataSource.insert(COLLECTION, DEPLOYS)
    Map found = (Map) dataSource.findById(COLLECTION, id)
    Map patched = (Map) dataSource.patch(COLLECTION, id, CHANGE)
    Map<String, Object> report = countPages()
    report.found = found.kind
    report.patched = patched.count
    report.job = runJob()
    dataSource.deleteById(COLLECTION, id)
    report.gone = dataSource.findById(COLLECTION, id) == null
    report.dropped = dataSource.dropCollection(COLLECTION)
    return report
  }

  private Map<String, Object> countPages() {
    int pages = 0
    int seen = 0
    String cursor = null
    boolean more = true
    while (more) {
      Map window = (Map) dataSource.scroll(COLLECTION, DEPLOYS, cursor, PAGE_SIZE)
      pages += 1
      seen += ((List) window.content).size()
      cursor = (String) window.nextCursor
      more = cursor != null
    }
    return [pages: pages, seen: seen]
  }

  private Object runJob() {
    dataSource.publish(TOPIC, [app: "web"], "render:web")
    Map delivery = (Map) ((List) dataSource.claim(TOPIC, GROUP, 1, 30000)).get(0)
    dataSource.ack(TOPIC, GROUP, delivery)
    return delivery.payload
  }
}
