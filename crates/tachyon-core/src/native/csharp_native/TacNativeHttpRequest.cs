internal sealed class TacNativeHttpRequest
{
    private const int RequestLimit = 1048576;
    private const int ResponseLimit = 1048576;
    private const int PathLimit = 2048;
    private const int AddressLimit = 8192;
    private const int QueryNameLimit = 256;
    private const int TextLimit = 8192;
    private const int HeaderCountLimit = 128;
    private const int HeaderNameLimit = 128;
    private const long DeadlineReach = 60000;
    private const string HeaderNameMarks = "!#$%&'*+.^_`|~-";
    private const string RefusedPathMarks = "\\?#%\uFEFF";
    private const string FormSafeMarks = "*-._";
    private static readonly string[] Methods = { "GET", "HEAD", "POST", "PUT", "PATCH", "DELETE" };
    private static readonly string[] Keys =
    {
        "method", "path", "query", "headers", "body", "redirect", "credentials", "deadline", "maxResponseBytes", "publicRead", "revalidate",
    };
    private static readonly string[] RefusedHeaders = { "host", "connection", "content-length", "transfer-encoding", "cookie", "origin", "referer" };

    internal readonly string Method;
    internal readonly string Address;
    internal readonly Dictionary<string, string> Headers;
    internal readonly string Body;
    internal readonly long Deadline;
    internal readonly int Limit;

    private TacNativeHttpRequest(JsonObject request, string origin)
    {
        Method = Text(request["method"]);
        Deadline = (long)Number(request["deadline"]);
        Limit = (int)Number(request["maxResponseBytes"]);
        Address = ReadAddress(request, origin);
        Headers = ReadHeaders(request);
        Body = ReadBody(request, Method, Headers);
    }

    internal static TacNativeHttpRequest Read(string text, string origin)
    {
        var request = Parse(text);
        CheckKeys(request);
        CheckEnvelope(request);
        CheckLimit(request["maxResponseBytes"]);
        CheckFlag(request, "publicRead");
        CheckFlag(request, "revalidate");
        return new TacNativeHttpRequest(request, origin);
    }

    internal System.Net.Http.HttpRequestMessage Message()
    {
        var message = new System.Net.Http.HttpRequestMessage(new System.Net.Http.HttpMethod(Method), Address);
        if (Body != null) message.Content = new System.Net.Http.ByteArrayContent(Encoding.UTF8.GetBytes(Body));
        foreach (var header in Headers) AddHeader(message, header.Key, header.Value);
        return message;
    }

    private static void AddHeader(System.Net.Http.HttpRequestMessage message, string name, string value)
    {
        if (message.Headers.TryAddWithoutValidation(name, value)) return;
        if (message.Content != null && message.Content.Headers.TryAddWithoutValidation(name, value)) return;
        throw new TacNativeHttpFailure("HTTP_HEADER");
    }

    private static JsonObject Parse(string text)
    {
        if (text == null || Encoding.UTF8.GetByteCount(text) > RequestLimit) throw new TacNativeHttpFailure("HTTP_REQUEST");
        try { return JsonNode.Parse(text) as JsonObject ?? throw new TacNativeHttpFailure("HTTP_REQUEST"); }
        catch (System.Text.Json.JsonException) { throw new TacNativeHttpFailure("HTTP_REQUEST"); }
    }

    private static void CheckKeys(JsonObject request)
    {
        foreach (var pair in request)
        {
            if (Array.IndexOf(Keys, pair.Key) < 0) throw new TacNativeHttpFailure("HTTP_REQUEST");
        }
    }

    private static void CheckEnvelope(JsonObject request)
    {
        var knownMethod = Array.IndexOf(Methods, Text(request["method"])) >= 0;
        var safeTransfer = Text(request["redirect"]) == "error" && Text(request["credentials"]) == "omit";
        var deadline = request["deadline"];
        var latest = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds() + DeadlineReach;
        var knownDeadline = TacNativeJson.IsSafeInteger(deadline) && Number(deadline) <= latest;
        if (!knownMethod || !safeTransfer || !knownDeadline) throw new TacNativeHttpFailure("HTTP_REQUEST");
    }

    private static void CheckLimit(JsonNode limit)
    {
        var whole = TacNativeJson.IsNumber(limit, out var number) && Math.Floor(number) == number;
        if (!whole || number < 1 || number > ResponseLimit) throw new TacNativeHttpFailure("HTTP_LIMIT");
    }

    private static void CheckFlag(JsonObject request, string name)
    {
        if (!request.TryGetPropertyValue(name, out var flag)) return;
        var kind = flag?.GetValueKind();
        if (kind != System.Text.Json.JsonValueKind.True && kind != System.Text.Json.JsonValueKind.False) throw new TacNativeHttpFailure("HTTP_REQUEST");
    }

    private static string ReadAddress(JsonObject request, string origin)
    {
        var path = Text(request["path"]);
        CheckPath(path);
        var query = ReadQuery(request["query"]);
        var written = query.Length == 0 ? origin + path : origin + path + "?" + query;
        string address;
        try { address = new Uri(written).AbsoluteUri; }
        catch (UriFormatException) { throw new TacNativeHttpFailure("HTTP_ORIGIN"); }
        if (Encoding.UTF8.GetByteCount(address) > AddressLimit) throw new TacNativeHttpFailure("HTTP_QUERY");
        return address;
    }

    private static void CheckPath(string path)
    {
        if (path == null || !path.StartsWith('/') || path.StartsWith("//")) throw new TacNativeHttpFailure("HTTP_ORIGIN");
        var refusedMark = path.Any(character => char.IsWhiteSpace(character) || RefusedPathMarks.Contains(character));
        var dotPart = path.Split('/').Any(part => part == "." || part == "..");
        if (refusedMark || dotPart || Encoding.UTF8.GetByteCount(path) > PathLimit) throw new TacNativeHttpFailure("HTTP_ORIGIN");
    }

    private static string ReadQuery(JsonNode query)
    {
        if (query == null) return "";
        if (query is not JsonObject values) throw new TacNativeHttpFailure("HTTP_QUERY");
        var pairs = new List<string>();
        foreach (var pair in values) pairs.Add(FormText(pair.Key) + "=" + FormText(QueryValue(pair.Key, pair.Value)));
        return string.Join("&", pairs);
    }

    private static string QueryValue(string name, JsonNode value)
    {
        var kind = value?.GetValueKind();
        var plain = kind == System.Text.Json.JsonValueKind.String || kind == System.Text.Json.JsonValueKind.Number
            || kind == System.Text.Json.JsonValueKind.True || kind == System.Text.Json.JsonValueKind.False;
        if (name.Length > QueryNameLimit || !plain) throw new TacNativeHttpFailure("HTTP_QUERY");
        var text = QueryText(value, kind.Value);
        var written = kind == System.Text.Json.JsonValueKind.String ? Encoding.UTF8.GetByteCount(text) + 2 : text.Length;
        if (written > TextLimit) throw new TacNativeHttpFailure("HTTP_QUERY");
        TacNativeJson.Check(value);
        return text;
    }

    private static string QueryText(JsonNode value, System.Text.Json.JsonValueKind kind)
    {
        if (kind == System.Text.Json.JsonValueKind.String) return value.GetValue<string>();
        if (kind == System.Text.Json.JsonValueKind.True) return "true";
        if (kind == System.Text.Json.JsonValueKind.False) return "false";
        return NumberText(value);
    }

    private static string NumberText(JsonNode value)
    {
        if (!TacNativeJson.IsNumber(value, out var number) || !double.IsFinite(number)) return value.ToJsonString();
        if (Math.Floor(number) == number && Math.Abs(number) < 1e21) return ((decimal)number).ToString(CultureInfo.InvariantCulture);
        var text = number.ToString("R", CultureInfo.InvariantCulture);
        var split = text.IndexOf('E');
        if (split < 0) return text;
        var exponent = int.Parse(text[(split + 1)..], CultureInfo.InvariantCulture);
        return text[..split] + "e" + (exponent < 0 ? "-" : "+") + Math.Abs(exponent).ToString(CultureInfo.InvariantCulture);
    }

    private static string FormText(string text)
    {
        var encoded = new StringBuilder();
        foreach (var value in Encoding.UTF8.GetBytes(text))
        {
            var character = (char)value;
            if (char.IsAsciiLetterOrDigit(character) || FormSafeMarks.Contains(character)) encoded.Append(character);
            else if (character == ' ') encoded.Append('+');
            else encoded.Append('%').Append(value.ToString("X2", CultureInfo.InvariantCulture));
        }
        return encoded.ToString();
    }

    private static Dictionary<string, string> ReadHeaders(JsonObject request)
    {
        var headers = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        if (!request.TryGetPropertyValue("headers", out var given)) return headers;
        if (given is not JsonObject values || values.Count > HeaderCountLimit) throw new TacNativeHttpFailure("HTTP_HEADER");
        foreach (var pair in values) headers[pair.Key] = HeaderValue(pair.Key, pair.Value);
        return headers;
    }

    private static string HeaderValue(string name, JsonNode given)
    {
        var lower = name.ToLowerInvariant();
        var refusedName = !IsHeaderName(name) || Array.IndexOf(RefusedHeaders, lower) >= 0 || lower.StartsWith("sec-") || lower.StartsWith("proxy-");
        var value = Text(given);
        var refusedValue = value == null || value.IndexOfAny(new[] { '\r', '\n', '\0' }) >= 0 || Encoding.UTF8.GetByteCount(value) > TextLimit;
        if (refusedName || refusedValue) throw new TacNativeHttpFailure("HTTP_HEADER");
        return value;
    }

    private static bool IsHeaderName(string name)
    {
        if (name.Length == 0 || name.Length > HeaderNameLimit) return false;
        return name.All(character => char.IsAsciiLetterOrDigit(character) || HeaderNameMarks.Contains(character));
    }

    private static string ReadBody(JsonObject request, string method, Dictionary<string, string> headers)
    {
        if (!request.TryGetPropertyValue("body", out var body)) return null;
        if (method == "GET" || method == "HEAD") throw new TacNativeHttpFailure("HTTP_BODY");
        TacNativeJson.Check(body);
        headers["content-type"] = "application/json";
        return body?.ToJsonString() ?? "null";
    }

    private static string Text(JsonNode node) =>
        node is JsonValue value && value.GetValueKind() == System.Text.Json.JsonValueKind.String ? value.GetValue<string>() : null;

    private static double Number(JsonNode node) => TacNativeJson.IsNumber(node, out var number) ? number : 0;
}
