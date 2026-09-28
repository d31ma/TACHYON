using System.Collections.Generic;
using Models;

[RequestRoute("/products")]
[Public]
class ProductsController {
  [Needs]
  [Named("ProductsService")]
  ProductsService _catalog;

  [GetRoute]
  public List<Product> GetProducts() { return _catalog.list(); }

  [GetRoute("/{id}")]
  public Product GetProduct([FromPath] long id) { return _catalog.find(id); }
}
