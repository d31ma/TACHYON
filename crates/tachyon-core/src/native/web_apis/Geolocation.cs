public sealed class Geolocation
{
    public void CurrentPosition() => TacWebHost.Call("geolocation", "currentPosition", new JsonArray());
}

