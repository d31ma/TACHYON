public sealed class Blobs
{
    public void SaveHeld(string fileId, string key) => TacWebHost.Call("blobs", "saveHeld", new JsonArray(fileId, key));

    public void Download(string url, string key, string headers) => TacWebHost.Call("blobs", "download", new JsonArray(url, key, headers));

    public void List(string prefix) => TacWebHost.Call("blobs", "list", new JsonArray(prefix));

    public void Read(string key, long offset, int count) => TacWebHost.Call("blobs", "read", new JsonArray(key, offset, count));

    public void Remove(string key) => TacWebHost.Call("blobs", "remove", new JsonArray(key));

    public string Url(string key) => TacWebHost.Call("blobs", "url", new JsonArray(key)).GetValue<string>();

    public void Open(string key) => TacWebHost.Call("blobs", "open", new JsonArray(key));

    public void Close(string key) => TacWebHost.Call("blobs", "close", new JsonArray(key));

    public string Upload(string key, string method, string url, string headers) => TacWebHost.Call("blobs", "upload", new JsonArray(key, method, url, headers)).GetValue<string>();

    public void Cancel(string upload) => TacWebHost.Call("blobs", "cancel", new JsonArray(upload));
}
