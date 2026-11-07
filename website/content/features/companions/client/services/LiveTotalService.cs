using Models;

[Service]
public sealed class LiveTotalService {
    public int Seen = 0;

    [OnEvent({"replay":true})]
    public void Changed(CartTotal changed) { Seen = changed.Total; }
}
