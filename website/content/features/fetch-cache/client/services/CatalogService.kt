@Service
class CatalogService {
    @Needs
    private val products: ProductsClient
    var count: Int = 0

    fun load() {
        count = ((products.list() as Map<*, *>)["items"] as List<*>).size
    }

    fun add() {
        products.save(mapOf("name" to "anvil"))
    }
}
