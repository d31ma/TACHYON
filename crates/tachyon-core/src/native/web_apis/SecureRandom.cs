public sealed class SecureRandom
{
    public byte[] Bytes(int count) => ToBytes(TacWebHost.Call("secureRandom", "bytes", new JsonArray(count)).AsArray());

    public string Uuid() => TacWebHost.Call("secureRandom", "uuid", new JsonArray()).GetValue<string>();

    private static byte[] ToBytes(JsonArray values) => values.Select(value => (byte)value.GetValue<int>()).ToArray();
}

