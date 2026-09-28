import java.util.List;
import models.Product;

@RequestRoute("/products")
@Public
class ProductsController {
    @GetRoute
    List<Product> getProducts() {
        return List.of(new Product("anvil"));
    }

    @PostRoute
    @ResponseStatus(201)
    Product postProduct(@FromBody Product product) {
        return product;
    }
}
