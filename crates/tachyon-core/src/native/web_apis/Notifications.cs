public sealed class Notifications
{
    private const string Api = "notifications";

    public string Permission() => TacWebHost.Call(Api, "permission", new JsonArray()).GetValue<string>();

    public void RequestPermission() => TacWebHost.Call(Api, "requestPermission", new JsonArray());

    public void Show(string title, string body) => TacWebHost.Call(Api, "show", new JsonArray(title, body));
}

