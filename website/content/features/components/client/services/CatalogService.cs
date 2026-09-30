using System.Collections.Generic;
using Models;

[Service]
public class CatalogService
{
    public List<Product> Products = new()
    {
        new Product("anvil", "Anvil", "£42.00"),
        new Product("rope", "Rope", "£8.00"),
    };
}
