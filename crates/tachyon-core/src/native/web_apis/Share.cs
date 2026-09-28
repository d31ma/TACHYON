public sealed class Share
{
    public void Open(string title, string text, string url) => TacWebHost.Call("share", "open", new JsonArray(title, text, url));
}

