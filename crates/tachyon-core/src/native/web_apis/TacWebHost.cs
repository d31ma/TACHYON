public static class TacWebHost
{
    public static Func<string, string> Ask;

    public static JsonNode Call(string api, string call, JsonArray arguments)
    {
        var request = new JsonObject { ["api"] = api, ["call"] = call, ["args"] = arguments };
        var answer = JsonNode.Parse(Send(request.ToJsonString())) as JsonObject;
        return ValueOf(answer);
    }

    private static string Send(string request)
    {
        if (Ask == null)
        {
            return "{\"error\":\"This browser API needs the page, and this service runs without one.\"}";
        }
        return Ask(request);
    }

    private static JsonNode ValueOf(JsonObject answer)
    {
        if (answer?["error"] is JsonNode error)
        {
            throw new InvalidOperationException(error.GetValue<string>());
        }
        return answer?["value"];
    }
}

