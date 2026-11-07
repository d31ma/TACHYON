[Service]
public class TallyService
{
    public int Count = 0;

    public void Add(int amount) => Count += amount;

    public string Summary(string unit) => Count + " " + unit;
}
