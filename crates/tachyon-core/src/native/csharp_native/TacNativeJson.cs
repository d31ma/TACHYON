internal static class TacNativeJson
{
    private const int DepthLimit = 32;
    private const double SafeIntegerLimit = 9007199254740991;

    internal static void Check(JsonNode node, int depth = 0)
    {
        if (depth > DepthLimit) throw new TacNativeHttpFailure("HTTP_RESPONSE");
        switch (node)
        {
            case JsonObject map:
                foreach (var pair in map) Check(pair.Value, depth + 1);
                return;
            case JsonArray list:
                foreach (var item in list) Check(item, depth + 1);
                return;
            case JsonValue value:
                CheckNumber(value);
                return;
        }
    }

    internal static bool IsNumber(JsonNode node, out double number)
    {
        number = 0;
        if (node is not JsonValue value || value.GetValueKind() != System.Text.Json.JsonValueKind.Number) return false;
        return value.TryGetValue(out number);
    }

    internal static bool IsSafeInteger(JsonNode node) =>
        IsNumber(node, out var number) && Math.Floor(number) == number && Math.Abs(number) <= SafeIntegerLimit;

    private static void CheckNumber(JsonValue value)
    {
        if (value.GetValueKind() != System.Text.Json.JsonValueKind.Number) return;
        if (!IsNumber(value, out var number)) throw new TacNativeHttpFailure("HTTP_RESPONSE");
        if (!double.IsFinite(number) || Math.Abs(number) > SafeIntegerLimit) throw new TacNativeHttpFailure("HTTP_RESPONSE");
    }
}
