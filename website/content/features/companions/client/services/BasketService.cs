[Service]
public sealed class BasketService {
    public int Count = 0;

    public int Doubled() => Count * 2;

    [OnStart]
    public void Initialize() { Count = 6; }
}
