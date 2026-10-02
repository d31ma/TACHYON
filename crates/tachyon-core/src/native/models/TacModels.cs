public static partial class TacModels
{
    public static T ReadResult<T>(string json, string shape, string models) =>
        ReadVerifiedResult<T>(json, shape, models);

    private static T ReadVerifiedResult<T>(string json, string shape, string models)
    {
        RequireResultBound(json);
        var options = new System.Text.Json.JsonDocumentOptions { MaxDepth = 32 };
        var value = System.Text.Json.Nodes.JsonNode.Parse(json, null, options);
        var schema = System.Text.Json.Nodes.JsonNode.Parse(models, null, options);
        RequireResultShape(schema, shape, PlainResult(value));
        var information = TacModelContext.Default.GetTypeInfo(typeof(T));
        if (information == null) throw new System.ArgumentException("Unknown model result.");
        return (T)System.Text.Json.JsonSerializer.Deserialize(json, information);
    }

    private static void RequireResultBound(string json)
    {
        if (System.Text.Encoding.UTF8.GetByteCount(json) > 1_048_576)
            throw new System.ArgumentException("Model result exceeds its bound.");
    }

    private static void RequireResultShape(System.Text.Json.Nodes.JsonNode models, string shape, object value)
    {
        if (!Fits(models, shape, value)) throw new System.ArgumentException("Invalid model result.");
    }

    private static object PlainResult(System.Text.Json.Nodes.JsonNode value) => value switch
    {
        System.Text.Json.Nodes.JsonObject map => map.ToDictionary(entry => entry.Key, entry => PlainResult(entry.Value)),
        System.Text.Json.Nodes.JsonArray items => items.Select(PlainResult).ToArray(),
        System.Text.Json.Nodes.JsonValue scalar => PlainResultScalar(scalar),
        _ => null
    };

    private static object PlainResultScalar(System.Text.Json.Nodes.JsonValue scalar)
    {
        var value = scalar.GetValue<System.Text.Json.JsonElement>();
        return value.ValueKind switch
        {
            System.Text.Json.JsonValueKind.String => value.GetString(),
            System.Text.Json.JsonValueKind.True => true,
            System.Text.Json.JsonValueKind.False => false,
            System.Text.Json.JsonValueKind.Number => PlainResultNumber(value),
            _ => null
        };
    }

    private static object PlainResultNumber(System.Text.Json.JsonElement value) =>
        value.TryGetInt64(out var integer) ? (object)integer : value.GetDouble();

    public static bool Fits(System.Text.Json.Nodes.JsonNode models, string shape, object value)
    {
        if (shape.StartsWith("?")) return value == null || Fits(models, shape.Substring(1), value);
        if (shape.StartsWith("[]"))
        {
            if (value is not object[] items) return false;
            return items.All(item => Fits(models, shape.Substring(2), item));
        }
        switch (shape)
        {
            case "any": return true;
            case "string": return value is string;
            case "integer": return value is int || value is long;
            case "number": return value is int || value is long || value is double;
            case "boolean": return value is bool;
        }
        if (models?[shape] is not System.Text.Json.Nodes.JsonObject model || value is not IDictionary<string, object> map) return false;
        if (model["fields"] is not System.Text.Json.Nodes.JsonObject fields) return false;
        bool open = model["open"] is System.Text.Json.Nodes.JsonValue declared && declared.TryGetValue<bool>(out var flag) && flag;
        if (!open && map.Keys.Any(key => !fields.ContainsKey(key))) return false;
        foreach (var entry in fields)
        {
            var field = entry.Value?.GetValue<string>() ?? "";
            bool required = field.StartsWith("!");
            if (!map.TryGetValue(entry.Key, out var found)) { if (required) return false; continue; }
            if (!Fits(models, required ? field.Substring(1) : field, found)) return false;
        }
        return true;
    }
}
