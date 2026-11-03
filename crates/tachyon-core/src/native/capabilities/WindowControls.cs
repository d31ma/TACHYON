public sealed class WindowControls
{
    private readonly NativeCapabilityOwner owner;
    internal WindowControls(NativeCapabilityOwner owner) { this.owner = owner; }

    public bool Granted(string control)
    {
        try { return System.Text.Json.Nodes.JsonNode.Parse(owner.Answer("window.granted", new() { ["control"] = control }))?["granted"]?.GetValue<bool>() == true; }
        catch (Exception) { return false; }
    }

    public void Minimize() => Invoke("minimize");
    public void Maximize() => Invoke("maximize");
    public void Fullscreen() => Invoke("fullscreen");
    public void Close() => Invoke("close");
    public void SetTitle(string title)
    {
        if (title == null || title.Length > 512 || title.IndexOf('\0') >= 0) throw new ArgumentException("Native window title is invalid.");
        Invoke("title", new() { ["title"] = title });
    }
    public void Resize(int width, int height)
    {
        if (width < 64 || width > 16384 || height < 64 || height > 16384) throw new ArgumentException("Native window size is invalid.");
        Invoke("resize", new() { ["width"] = width, ["height"] = height });
    }

    private void Invoke(string control, System.Text.Json.Nodes.JsonObject fields = null)
    {
        if (!Granted(control)) throw new InvalidOperationException($"The {control} window control is not granted. Add \"{control}\" to tac.application.window.controls in client/manifest.json.");
        try
        {
            if (System.Text.Json.Nodes.JsonNode.Parse(owner.Answer("window." + control, fields ?? new()))?["ok"]?.GetValue<bool>() != true)
                throw new InvalidOperationException("Native window control is unavailable.");
        }
        catch (Exception) { throw new InvalidOperationException("Native window control is unavailable."); }
    }
}
