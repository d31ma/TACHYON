using System.Collections.Generic;

[Service]
public class BasketService
{
    public int Count = 0;
    public string Label = "";
    private string selected = "";
    public object Tabs = new List<Dictionary<string, string>>
    {
        new Dictionary<string, string> { ["id"] = "a", ["name"] = "A" },
    };

    public void Increment() => Count += 1;

    public void Select(string id) => selected = id;
}
