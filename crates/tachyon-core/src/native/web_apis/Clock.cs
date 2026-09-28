public sealed class Clock
{
    public double NowMs() => TacWebHost.Call("clock", "now", new JsonArray()).GetValue<double>();

    public double MonotonicMs() => TacWebHost.Call("clock", "monotonic", new JsonArray()).GetValue<double>();

    public string TimeZone() => TacWebHost.Call("clock", "timeZone", new JsonArray()).GetValue<string>();
}

