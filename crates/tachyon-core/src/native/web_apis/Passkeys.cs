public sealed class Passkeys
{
    private const string Api = "passkeys";

    public bool Supported() => TacWebHost.Call(Api, "supported", new JsonArray()).GetValue<bool>();

    public void Check() => TacWebHost.Call(Api, "check", new JsonArray());

    public void Create(string optionsJson) => TacWebHost.Call(Api, "create", new JsonArray(optionsJson));

    public void Get(string optionsJson) => TacWebHost.Call(Api, "get", new JsonArray(optionsJson));

    public void Offer(string optionsJson) => TacWebHost.Call(Api, "offer", new JsonArray(optionsJson));

    public void Cancel() => TacWebHost.Call(Api, "cancel", new JsonArray());
}
