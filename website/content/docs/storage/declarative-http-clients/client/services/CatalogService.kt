@Service
class CatalogService {
    @Needs
    private val catalog: CatalogClient
    var products: List<String> = listOf()
    var error: String = ""

    fun load() {
        show(runCatching { catalog.all() }.getOrNull())
    }

    private fun show(answer: Any?) {
        if (answer !is List<*>) return showFailure()
        products = answer.map { it.toString() }
        error = ""
    }

    private fun showFailure() {
        error = "Products could not be loaded. Please retry."
    }
}
