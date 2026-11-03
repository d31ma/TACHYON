import models.Product
import models.Detail
import models.Summary
@RequestRoute("/catalog")
@Public
class CatalogController {
    @Needs
    val converter: ProductConverter
    @GetRoute
    fun getCatalog(): Summary {
        val detail = Detail("detail", "detail-private")
        return converter.map(Product(7, "Anvil", detail, listOf(detail), "product-private"))
    }
}
