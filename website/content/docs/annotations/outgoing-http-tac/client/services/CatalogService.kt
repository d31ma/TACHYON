@Service
class CatalogService {
    private val catalog: CatalogClient = CatalogClient()
    var count: Int = 0

    fun all(): Any? = catalog.all()
}
