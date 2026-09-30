using Models;

[Service]
public class BasketService
{
    public int Count = 0;
    public string Label = "";
    private string selected = "";
    public List<Tab> Tabs = new List<Tab> { new Tab("a", "A") };

    public void Increment() => Count += 1;

    public void Select(string id) => selected = id;
}
