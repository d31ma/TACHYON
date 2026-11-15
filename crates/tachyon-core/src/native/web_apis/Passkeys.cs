public sealed class Passkeys
{
    private const string Api = "passkeys";

    public bool Supported() => TacWebHost.Call(Api, "supported", new JsonArray()).GetValue<bool>();

    public void Check() => TacWebHost.Call(Api, "check", new JsonArray());

    public void Create(string optionsJson, string prfKey) => TacWebHost.Call(Api, "create", new JsonArray(optionsJson, prfKey));

    public void Get(string optionsJson, string prfKey) => TacWebHost.Call(Api, "get", new JsonArray(optionsJson, prfKey));

    public void Offer(string optionsJson, string prfKey) => TacWebHost.Call(Api, "offer", new JsonArray(optionsJson, prfKey));

    public void Cancel() => TacWebHost.Call(Api, "cancel", new JsonArray());
}
