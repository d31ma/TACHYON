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

    public System.Threading.Tasks.Task<Models.NullableGreeting> nullable()
    {
        return _greetingClient.nullable();
    }

    public System.Threading.Tasks.Task<Models.Greeting> missing()
    {
        return _accept(_greetingClient.missing());
    }

    public System.Threading.Tasks.Task<Models.Greeting> wrongType()
    {
        return _accept(_greetingClient.wrongType());
    }

    public System.Threading.Tasks.Task<Models.Greeting> extra()
    {
        return _accept(_greetingClient.extra());
    }

    public System.Threading.Tasks.Task<Models.Greeting> slow()
    {
        return _accept(_greetingClient.slow());
    }

    private async System.Threading.Tasks.Task<Models.Greeting> _accept(System.Threading.Tasks.Task<Models.Greeting> pending)
    {
        var greeting = await pending;
        Message = greeting.Message;
        return greeting;
    }
}
