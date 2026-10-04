@Service
class EventService {
  @Needs
  val dataSource: DataSource

  fun report(): Map<String, Any?> {
    dataSource.createCollection(COLLECTION)
    val id = (dataSource.insert(COLLECTION, DEPLOYS) as Map<*, *>)["id"] as String
    dataSource.insert(COLLECTION, DEPLOYS)
    dataSource.insert(COLLECTION, DEPLOYS)
    val found = dataSource.findById(COLLECTION, id) as Map<*, *>
    val patched = dataSource.patch(COLLECTION, id, CHANGE) as Map<*, *>
    val scrolled = countPages()
    val job = runJob()
    dataSource.deleteById(COLLECTION, id)
    val gone = dataSource.findById(COLLECTION, id) == null
    val dropped = dataSource.dropCollection(COLLECTION)
    return scrolled + mapOf("found" to found["kind"], "patched" to patched["count"], "job" to job, "gone" to gone, "dropped" to dropped)
  }

  private fun countPages(): Map<String, Int> {
    var pages = 0
    var seen = 0
    var cursor: String? = null
    var more = true
    while (more) {
      val window = dataSource.scroll(COLLECTION, DEPLOYS, cursor, PAGE_SIZE) as Map<*, *>
      pages += 1
      seen += (window["content"] as List<*>).size
      cursor = window["nextCursor"] as String?
      more = cursor != null
    }
    return mapOf("pages" to pages, "seen" to seen)
  }

  private fun runJob(): Any? {
    dataSource.publish(TOPIC, mapOf("app" to "web"), "render:web")
    val delivery = (dataSource.claim(TOPIC, GROUP, 1, 30000) as List<*>).first() as Map<*, *>
    dataSource.ack(TOPIC, GROUP, delivery)
    return delivery["payload"]
  }

  companion object {
    private const val COLLECTION = "hd-events"
    private val DEPLOYS = mapOf("kind" to "deploy")
    private val CHANGE = mapOf("count" to 7)
    private const val PAGE_SIZE = 2
    private const val TOPIC = "app.render"
    private const val GROUP = "renderers"
  }
}
