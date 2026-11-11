public sealed class Push
{
    private const string Api = "push";

    public bool Supported() => TacWebHost.Call(Api, "supported", new JsonArray()).GetValue<bool>();

    public void Subscribe(string applicationServerKey) => TacWebHost.Call(Api, "subscribe", new JsonArray(applicationServerKey));

    public void Subscription() => TacWebHost.Call(Api, "subscription", new JsonArray());

    public void Unsubscribe() => TacWebHost.Call(Api, "unsubscribe", new JsonArray());
}
