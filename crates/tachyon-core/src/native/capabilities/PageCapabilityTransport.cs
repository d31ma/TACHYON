internal static class PageCapabilityTransport
{
    private const string Unsupported = "{\"status\":\"unsupported\"}";

    internal static string Send(string capability, string request)
    {
        try { return TacWebHost.Call("nativeCapability", "call", new System.Text.Json.Nodes.JsonArray("csharp", capability, request))?.GetValue<string>() ?? Unsupported; }
        catch (Exception) { return Unsupported; }
    }
}
