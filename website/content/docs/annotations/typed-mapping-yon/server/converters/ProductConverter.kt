import models.Product
import models.Summary
@Converter
interface ProductConverter {
@Convert({"fields":[{"target":"kind","constant":"product"}]})
fun map(product: Product): Summary
}
