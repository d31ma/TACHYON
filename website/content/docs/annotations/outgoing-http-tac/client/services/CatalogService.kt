@Service
class CatalogService {
    private val catalog: CatalogClient = CatalogClient()
    var count: Int = 0

    fun load() {
        val answer = catalog.all() as Map<*, *>
        count = (answer["items"] as List<*>).size
    }
}
