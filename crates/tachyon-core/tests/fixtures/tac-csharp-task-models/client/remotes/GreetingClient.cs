[HTTPClient({"path":"/api/greeting","adapter":"greeting","setup":"GreetingSetup"})]
public interface GreetingClient
{
    [GetRequest]
    System.Threading.Tasks.Task<Models.Greeting> read();

    [GetRequest("/nullable")]
    System.Threading.Tasks.Task<Models.NullableGreeting> nullable();

    [GetRequest("/missing")]
    System.Threading.Tasks.Task<Models.Greeting> missing();

    [GetRequest("/wrong-type")]
    System.Threading.Tasks.Task<Models.Greeting> wrongType();

    [GetRequest("/extra")]
    System.Threading.Tasks.Task<Models.Greeting> extra();

    [GetRequest("/slow")]
    System.Threading.Tasks.Task<Models.Greeting> slow();
}
