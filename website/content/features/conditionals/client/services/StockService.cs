using System.Collections.Generic;

[Service]
public class StockService
{
    public object Products = new List<Dictionary<string, string>>();
    public bool Restocking = true;
}
