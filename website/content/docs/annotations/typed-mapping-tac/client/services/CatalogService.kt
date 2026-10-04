import models.Detail
import models.Product
import models.Summary
@Service
class CatalogService {
    @Needs
    lateinit var productConverter: ProductConverter
    var result: String = ""

    fun load() {
        val detail = Detail("detail", "detail-private")
        val product = productConverter.copy(Product(7, "Anvil", detail, listOf(detail), "product-private"))
        result = text(productConverter.map(product))
    }

    private fun text(summary: Summary): String {
        return "${summary.kind} ${summary.id}: ${summary.name}, ${summary.detail.name}, ${summary.items.size}"
    }
}
