using Models;

[Service]
public sealed class HeardService
{
    private readonly ApplicationEventPublisher applicationEventPublisher;

    public string Heard = "";

    public void Send(int step)
    {
        applicationEventPublisher.PublishEvent(new Ping(step));
    }

    [OnEvent]
    public async System.Threading.Tasks.Task Later(Ping ping)
    {
        await System.Threading.Tasks.Task.Delay(20);
        Heard += "a" + ping.Step;
    }

    [OnEvent]
    public void Now(Ping ping)
    {
        Heard += "s" + ping.Step;
    }
}
