[Service]
public class CounterService
{
    public int Count = 0;
    [SessionState]
    public int Visits = 0;
    [LocalState]
    public string Theme = "light";

    public void Increment() => Count += 1;
}
