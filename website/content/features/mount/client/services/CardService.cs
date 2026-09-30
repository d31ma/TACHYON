[Service]
public class CardService
{
    public string Name = "Anvil";
    public string Status = "waiting";

    [OnStart]
    public void Activate() { Status = "active"; }
}
