[Service]
public sealed class GreetingService
{
    [Needs]
    private readonly GreetingClient _greetingClient;

    public string Message = "";

    public async System.Threading.Tasks.Task<Models.Greeting> ask()
    {
        var greeting = await _greetingClient.read();
        Message = greeting.Message;
        return greeting;
    }

    public System.Threading.Tasks.Task<Models.NullableGreeting> nullable()
    {
        return _greetingClient.nullable();
    }
}
