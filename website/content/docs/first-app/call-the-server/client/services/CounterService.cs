[Service]
public sealed class CounterService
{
    [Needs]
    private readonly GreetingClient _greetingClient;

    public int Count = 0;
    public string Message = "";

    public void Increment() => Count += 1;

    public System.Threading.Tasks.Task<Models.Greeting> AskServer()
    {
        return _accept(_greetingClient.getGreeting());
    }

    private async System.Threading.Tasks.Task<Models.Greeting> _accept(System.Threading.Tasks.Task<Models.Greeting> pending)
    {
        var greeting = await pending;
        Message = greeting.Message;
        return greeting;
    }
}
