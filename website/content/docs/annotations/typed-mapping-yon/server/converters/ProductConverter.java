import models.Product;
import models.Summary;
@Converter
interface ProductConverter {
@Convert({"fields":[{"target":"kind","constant":"product"}]})
Summary map(Product product);
}
