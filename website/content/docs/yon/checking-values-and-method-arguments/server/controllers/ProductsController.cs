using System.Collections.Generic;
using Models;

[RequestRoute("/products")]
[Public]
class ProductsController {
    [Needs]
    readonly ProductService _productService;

    [PostRoute]
    public Product PostProduct([FromBody] Product product) {
        _productService.accept(BehaviorJSON.Object(new Dictionary<string, BehaviorJSON> {
            ["name"] = BehaviorJSON.From(product.Name),
            ["level"] = BehaviorJSON.From(product.Level),
        }));
        return product;
    }
}
