public sealed class Preferences
{
    public string Scheme() => Text("scheme");

    public string Motion() => Text("motion");

    public string Language() => Text("language");

    private static string Text(string call) => TacWebHost.Call("preferences", call, new JsonArray()).GetValue<string>();
}

