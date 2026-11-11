public sealed class Focus
{
    public void Move(string target) => TacWebHost.Call("focus", "move", new JsonArray(target));

    public void Trap(string container) => TacWebHost.Call("focus", "trap", new JsonArray(container));

    public void Release() => TacWebHost.Call("focus", "release", new JsonArray());
}

