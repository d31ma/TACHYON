public sealed class StorageSpace
{
    private const string Api = "storageSpace";

    public void Estimate() => TacWebHost.Call(Api, "estimate", new JsonArray());

    public void Persist() => TacWebHost.Call(Api, "persist", new JsonArray());
}
