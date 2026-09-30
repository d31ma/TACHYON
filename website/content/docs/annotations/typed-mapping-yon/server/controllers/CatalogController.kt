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
val detail = Detail("detail", "hidden")
return converter.map(Product(7, "Anvil", detail, listOf(detail), "hidden"))
}
}
