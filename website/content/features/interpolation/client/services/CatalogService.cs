using System.Collections.Generic;

[Service]
public class CatalogService
{
    public string Title = "Catalog";
    public int Count = 6;
    public object Product = new Dictionary<string, string> { ["name"] = "Anvil", ["price"] = "£42.00" };
}
