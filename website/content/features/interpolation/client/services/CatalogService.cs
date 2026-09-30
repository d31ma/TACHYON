using Models;

[Service]
public class CatalogService
{
    public string Title = "Catalog";
    public int Count = 6;
    public Product Product = new Product("Anvil", "£42.00");
}
