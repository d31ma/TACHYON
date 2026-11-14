public sealed class Peers
{
    private const string Api = "peers";

    public bool Supported() => TacWebHost.Call(Api, "supported", new JsonArray()).GetValue<bool>();

    public string Open(string username, string credential) => TacWebHost.Call(Api, "open", new JsonArray(username, credential)).GetValue<string>();

    public void Offer(string connection) => TacWebHost.Call(Api, "offer", new JsonArray(connection));

    public void Answer(string connection, string offerSdp) => TacWebHost.Call(Api, "answer", new JsonArray(connection, offerSdp));

    public void Accept(string connection, string answerSdp) => TacWebHost.Call(Api, "accept", new JsonArray(connection, answerSdp));

    public void AddCandidate(string connection, string candidateJson) => TacWebHost.Call(Api, "addCandidate", new JsonArray(connection, candidateJson));

    public void Restart(string connection) => TacWebHost.Call(Api, "restart", new JsonArray(connection));

    public string OpenChannel(string connection, string label, bool ordered) => TacWebHost.Call(Api, "openChannel", new JsonArray(connection, label, ordered)).GetValue<string>();

    public bool Send(string connection, string channel, string data) => TacWebHost.Call(Api, "send", new JsonArray(connection, channel, data)).GetValue<bool>();

    public void CloseChannel(string connection, string channel) => TacWebHost.Call(Api, "closeChannel", new JsonArray(connection, channel));

    public void Close(string connection) => TacWebHost.Call(Api, "close", new JsonArray(connection));
}
