public sealed class WindowControls
{
    private const string Api = "windowControls";

    public bool Granted(string control) => TacWebHost.Call(Api, "granted", new JsonArray(control)).GetValue<bool>();

    public void Minimize() => TacWebHost.Call(Api, "minimize", new JsonArray());

    public void Maximize() => TacWebHost.Call(Api, "maximize", new JsonArray());

    public void Fullscreen() => TacWebHost.Call(Api, "fullscreen", new JsonArray());

    public void Close() => TacWebHost.Call(Api, "close", new JsonArray());

    public void Resize(int width, int height) => TacWebHost.Call(Api, "resize", new JsonArray(width, height));

    public void SetTitle(string text) => TacWebHost.Call(Api, "setTitle", new JsonArray(text));
}
