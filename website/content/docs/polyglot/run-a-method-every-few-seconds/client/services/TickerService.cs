[Service]
public sealed class TickerService {
    public int Seconds = 0;

    [Every("1s")]
    public void Count() { Seconds += 1; }
}
