using System.Collections.Generic;
using Models;

[RequestRoute("/products")]
[Public]
class ProductsController {
    [GetRoute]
    public List<Product> GetProducts() {
        return new List<Product> { new Product("anvil") };
    }

    [PostRoute]
    [ResponseStatus(201)]
    public Product PostProduct([FromBody] Product product) {
        return product;
    }
}
