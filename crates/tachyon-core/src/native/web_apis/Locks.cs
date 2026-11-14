public sealed class Locks
{
    private const string Api = "locks";

    public void Request(string name) => TacWebHost.Call(Api, "request", new JsonArray(name));

    public void Release(string name) => TacWebHost.Call(Api, "release", new JsonArray(name));
}
