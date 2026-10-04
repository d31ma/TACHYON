[Service]
public sealed class GreetingService
{
    [Needs]
    private readonly GreetingClient _greetingClient;

    public string Message = "";

    public System.Threading.Tasks.Task<Models.Greeting> ask()
    {
        return _accept(_greetingClient.read());
    }

    private async System.Threading.Tasks.Task<Models.Greeting> _accept(System.Threading.Tasks.Task<Models.Greeting> pending)
    {
        var greeting = await pending;
        Message = greeting.Message;
        return greeting;
    }
}
