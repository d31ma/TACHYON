using System.Collections.Generic;
using Models;

[Service]
[Lifetime("transient")]
class ProductsService {
  public List<Product> list() { return new List<Product> { find(1) }; }
  public Product find(long id) { return new Product(id, "Anvil"); }
}
