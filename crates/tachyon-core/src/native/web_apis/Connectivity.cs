public sealed class Connectivity
{
    public bool Online() => TacWebHost.Call("connectivity", "online", new JsonArray()).GetValue<bool>();
}

