using System.Collections.Generic;

[Service]
public class CatalogService
{
    public object Products = new object[]
    {
        Product("anvil", "Anvil", "£42.00"),
        Product("rope", "Rope", "£8.00"),
    };

    private static Dictionary<string, object> Product(string id, string name, string price) =>
        new() { ["id"] = id, ["name"] = name, ["price"] = price };
}
