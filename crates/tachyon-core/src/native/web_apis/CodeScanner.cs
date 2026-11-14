public sealed class CodeScanner
{
    private const string Api = "codeScanner";

    public bool Supported() => TacWebHost.Call(Api, "supported", new JsonArray()).GetValue<bool>();

    public void Scan() => TacWebHost.Call(Api, "scan", new JsonArray());

    public void Cancel() => TacWebHost.Call(Api, "cancel", new JsonArray());
}
