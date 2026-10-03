public static class BrowserFakes
{
    private const string Missing = "This browser API needs the page. A test unit has no page: give the call an answer with BrowserFakes.Answer in the test unit.";
    private static readonly Dictionary<string, JsonNode> Answers = new();

    public static void Answer(string api, string call, object value)
    {
        Answers[api + "." + call] = System.Text.Json.JsonSerializer.SerializeToNode(value);
    }

    public static string Reply(string request)
    {
        var asked = JsonNode.Parse(request) as JsonObject;
        var key = asked?["api"]?.GetValue<string>() + "." + asked?["call"]?.GetValue<string>();
        if (!Answers.TryGetValue(key, out var value))
        {
            return new JsonObject { ["error"] = Missing }.ToJsonString();
        }
        return new JsonObject { ["value"] = value?.DeepClone() }.ToJsonString();
    }
}
