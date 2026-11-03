import java.util.List;
import models.Product;

@RequestRoute("/products")
@Public
class ProductsController {
  @Needs
  @Named("ProductsService")
  ProductsService catalog;

  @GetRoute
  List<Product> getProducts() { return catalog.list(); }

  @GetRoute("/{id}")
  Product getProduct(@FromPath long id) { return catalog.find(id); }
}
