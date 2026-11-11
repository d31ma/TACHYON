[Service]
public class AlertsService
{
    private const string ServerKey = "BEl62iUYgUivxIkv69yViEuiBIa-Ib9-SkvMeAtA3LFgDzkrxZJjSgSnfckjBJuBkr3qBUYIHBQFLXYp5Nksh8U";
    private readonly Push push = new();
    public string Endpoint = "";
    public string Problem = "";

    public void TurnOn() => KeepProblem(() => push.Subscribe(ServerKey));

    public void TurnOff() => KeepProblem(push.Unsubscribe);

    [OnEvent]
    public void Subscribed(PushSubscribed subscribed)
    {
        Endpoint = subscribed.Endpoint;
    }

    [OnEvent]
    public void Unsubscribed(PushUnsubscribed unsubscribed)
    {
        Endpoint = "";
    }

    [OnEvent]
    public void Failed(BrowserCallFailed failed)
    {
        Problem = failed.Message;
    }

    private void KeepProblem(Action call)
    {
        try
        {
            call();
        }
        catch (InvalidOperationException refusal)
        {
            Problem = refusal.Message;
        }
    }
}
