[HTTPClient({"path":"/api/greeting","adapter":"greeting","setup":"GreetingSetup"})]
public interface GreetingClient
{
    [GetRequest]
    System.Threading.Tasks.Task<Models.Greeting> getGreeting();
}
