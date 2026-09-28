public sealed class PageVisibility
{
    public bool Visible() => TacWebHost.Call("pageVisibility", "visible", new JsonArray()).GetValue<bool>();
}

