[Service]
public class TicketService
{
    private readonly Clock clock = new();
    private readonly SecureRandom secureRandom = new();
    public string Ticket = "";
    public double IssuedAt = 0;

    public void Issue()
    {
        Ticket = secureRandom.Uuid();
        IssuedAt = clock.NowMs();
    }
}
