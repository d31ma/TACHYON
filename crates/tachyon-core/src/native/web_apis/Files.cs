public sealed class Files
{
    public void Pick(string accept, bool multiple) => TacWebHost.Call("files", "pick", new JsonArray(accept, multiple));

    public void Read(string id, long offset, int count) => TacWebHost.Call("files", "read", new JsonArray(id, offset, count));

    public void Release(string id) => TacWebHost.Call("files", "release", new JsonArray(id));

    public string Upload(string id, string method, string url, string headers) => TacWebHost.Call("files", "upload", new JsonArray(id, method, url, headers)).GetValue<string>();

    public void Cancel(string upload) => TacWebHost.Call("files", "cancel", new JsonArray(upload));
}
