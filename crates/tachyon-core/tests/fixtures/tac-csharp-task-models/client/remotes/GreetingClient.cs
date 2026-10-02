[HTTPClient({"path":"/api/greeting","adapter":"greeting","setup":"GreetingSetup"})]
public interface GreetingClient
{
    [GetRequest]
    System.Threading.Tasks.Task<Models.Greeting> read();

    [GetRequest("/nullable")]
    System.Threading.Tasks.Task<Models.NullableGreeting> nullable();
}
