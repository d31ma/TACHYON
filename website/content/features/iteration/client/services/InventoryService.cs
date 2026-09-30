using System.Collections.Generic;

[Service]
public class InventoryService
{
    public object Products = new List<Dictionary<string, string>>
    {
        new Dictionary<string, string> { ["name"] = "Anvil" },
        new Dictionary<string, string> { ["name"] = "Rope" },
    };
}
