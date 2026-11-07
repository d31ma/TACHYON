public sealed class Clipboard
{
    private const string Api = "clipboard";

    public void WriteText(string text) => TacWebHost.Call(Api, "writeText", new JsonArray(text));

    public void ReadText() => TacWebHost.Call(Api, "readText", new JsonArray());
}

