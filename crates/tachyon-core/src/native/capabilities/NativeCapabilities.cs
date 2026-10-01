public static class NativeCapabilities
{
    public static Func<string, string, string> Call = (_, _) => "{\"status\":\"unsupported\"}";
    private static NativeCapabilityOwner constructing;
    private static readonly Dictionary<string, NativeCapabilityOwner> owners = new();

    public static T Create<T>(string owner, Func<T> work)
    {
        var previous = constructing;
        Dispose(owner);
        if (owners.Count >= 256) throw new ArgumentException("Too many native capability activations.");
        var activation = new NativeCapabilityOwner(owner, Call);
        owners[owner] = activation;
        constructing = activation;
        try { return work(); }
        catch { Dispose(owner); throw; }
        finally { constructing = previous; }
    }

    public static NativeCapabilityOwner Provider() => constructing ?? new("", Call);

    public static void Dispose(string owner)
    {
        if (owners.Remove(owner, out var activation)) activation.Dispose();
    }
}
