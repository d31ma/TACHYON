public sealed class NativeCapabilityOwner
{
    private readonly string owner;
    private readonly Func<string, string, string> call;
    private readonly int thread = Environment.CurrentManagedThreadId;
    private bool active = true;

    internal NativeCapabilityOwner(string owner, Func<string, string, string> call)
    {
        this.owner = owner;
        this.call = call;
    }

    internal string Answer(string capability, System.Text.Json.Nodes.JsonObject fields)
    {
        if (owner.Length == 0) return "{\"status\":\"unsupported\"}";
        if (!active) return "{\"status\":\"disposed\"}";
        if (thread != Environment.CurrentManagedThreadId) return "{\"status\":\"unavailable\"}";
        fields["owner"] = owner;
        try
        {
            var request = fields.ToJsonString();
            if (System.Text.Encoding.UTF8.GetByteCount(capability) > 64 || System.Text.Encoding.UTF8.GetByteCount(request) > 65536) return "{\"status\":\"invalid\"}";
            return call(capability, request);
        }
        catch (Exception) { return "{\"status\":\"unavailable\"}"; }
    }

    internal void Dispose()
    {
        if (!active) return;
        _ = Answer("filePicker.dispose", new());
        active = false;
    }
}
