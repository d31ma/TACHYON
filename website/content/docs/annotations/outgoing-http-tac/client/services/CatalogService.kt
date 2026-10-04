@Service
class CatalogService {
    @Needs
    private val catalog: CatalogClient
    var count: Int = 0

    fun load() {
        val answer = catalog.all() as Map<*, *>
        count = (answer["items"] as List<*>).size
    }
}
