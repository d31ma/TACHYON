using Models;

[Service]
public class InventoryService
{
    public List<Product> Products = new List<Product> { new Product("Anvil"), new Product("Rope") };
}
