internal sealed class TacNativeHttpFailure : Exception
{
    internal readonly string Code;
    internal readonly int Status;

    internal TacNativeHttpFailure(string code, int status = 0) : base("Native HTTP failed: " + code)
    {
        Code = code;
        Status = status;
    }

    internal string Envelope() => Code == "HTTP_STATUS"
        ? "{\"error\":{\"code\":\"HTTP_STATUS\",\"status\":" + Status.ToString(CultureInfo.InvariantCulture) + "}}"
        : Answer(Code);

    internal static string Answer(string code) => "{\"error\":{\"code\":\"" + code + "\"}}";
}
