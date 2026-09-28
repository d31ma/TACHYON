[Service]
public class TallyService
{
    public int Count = 6;
    public string Runtime => "C#, compiled by .NET";

    public int Doubled() => Count * 2;
}