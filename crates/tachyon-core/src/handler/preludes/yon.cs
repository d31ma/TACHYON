using System;
using System.Buffers.Binary;
using System.Collections;
using System.Collections.Generic;
using System.IO;
using System.Net.Http;
using System.Reflection;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Threading.Tasks;

sealed class DataSource
{
    public JsonElement Query(string sql, object[] parameters) => YonManagedResources.Call(new { operation = "sql", sql, parameters, mode = "rows" });
    public JsonElement Execute(string sql, object[] parameters) => YonManagedResources.Call(new { operation = "sql", sql, parameters, mode = "execute" });
    public JsonElement Execute(object command) => Command(command);
    public JsonElement Command(object command) => YonManagedResources.Call(command);
    public JsonElement query(string sql, object[] parameters) => Query(sql, parameters);
    public JsonElement execute(string sql, object[] parameters) => Execute(sql, parameters);
    public JsonElement execute(object command) => Command(command);
    public JsonElement command(object command) => Command(command);
}

sealed class RedisTemplate
{
    public JsonElement Get(string key) => YonManagedResources.Call(new { operation = "redis", command = "get", key });
    public JsonElement Delete(string key) => YonManagedResources.Call(new { operation = "redis", command = "delete", key });
    public JsonElement Set(string key, object value, long ttlSeconds) => YonManagedResources.Call(new { operation = "redis", command = "set", key, value, ttlSeconds });
    public JsonElement get(string key) => Get(key);
    public JsonElement delete(string key) => Delete(key);
    public JsonElement set(string key, object value, long ttlSeconds) => Set(key, value, ttlSeconds);
}

static class YonManagedResources
{
    internal const int Limit = 1024 * 1024;
    private const long ResourceTimeoutMilliseconds = 5000;
    private static InvalidOperationException Failed() => new("Yon managed resource operation failed.");
    internal static JsonElement Call(object command)
    {
        try
        {
            byte[] response = Exchange(JsonSerializer.SerializeToUtf8Bytes(command), ResourceTimeoutMilliseconds);
            using var document = JsonDocument.Parse(response);
            return document.RootElement.Clone();
        }
        catch (Exception) { throw Failed(); }
    }
    internal static byte[] Exchange(byte[] body, long timeoutMilliseconds)
    {
        try
        {
            var endpoint = Environment.GetEnvironmentVariable("YON_RESOURCE_ENDPOINT") ?? "";
            var token = Environment.GetEnvironmentVariable("YON_RESOURCE_TOKEN") ?? "";
            if (!System.Text.RegularExpressions.Regex.IsMatch(endpoint, "\\Ahttp://127\\.0\\.0\\.1:[0-9]{1,5}/\\z") ||
                !System.Text.RegularExpressions.Regex.IsMatch(token, "\\A[0-9a-f]{64}\\z")) throw Failed();
            int port = int.Parse(endpoint.Substring(17, endpoint.Length - 18), System.Globalization.CultureInfo.InvariantCulture);
            if (port < 1 || port > 65535) throw Failed();
            if (body.Length > Limit) throw Failed();
            int timeout = (int)Math.Clamp(timeoutMilliseconds, 1, int.MaxValue);
            var deadline = System.Diagnostics.Stopwatch.StartNew();
            using var socket = new System.Net.Sockets.TcpClient();
            socket.ConnectAsync(System.Net.IPAddress.Loopback, port).WaitAsync(TimeSpan.FromMilliseconds(timeout)).GetAwaiter().GetResult();
            using var stream = socket.GetStream();
            stream.WriteTimeout = timeout;
            string headers = $"POST / HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {token}\r\nContent-Type: application/json\r\nContent-Length: {body.Length}\r\nConnection: close\r\n\r\n";
            stream.Write(Encoding.ASCII.GetBytes(headers)); stream.Write(body); stream.Flush();
            using var head = new MemoryStream();
            byte[] delimiter = { 13, 10, 13, 10 };
            int matched = 0;
            while (matched < 4)
            {
                Timeout(stream, deadline, timeout);
                int next = stream.ReadByte();
                if (next < 0 || head.Length >= 16384) throw Failed();
                head.WriteByte((byte)next);
                matched = next == delimiter[matched] ? matched + 1 : next == 13 ? 1 : 0;
            }
            var lines = Encoding.ASCII.GetString(head.ToArray()).Split("\r\n");
            if (!System.Text.RegularExpressions.Regex.IsMatch(lines[0], "\\AHTTP/1\\.[01] 200(?: .*)?\\z")) throw Failed();
            int length = -1;
            for (int index = 1; index < lines.Length; index++)
            {
                var line = lines[index].ToLowerInvariant();
                if (line.StartsWith("transfer-encoding:", StringComparison.Ordinal)) throw Failed();
                if (line.StartsWith("content-length:", StringComparison.Ordinal))
                {
                    var value = line.Substring(15).Trim();
                    if (length >= 0 || !System.Text.RegularExpressions.Regex.IsMatch(value, "\\A[0-9]{1,7}\\z")) throw Failed();
                    length = int.Parse(value, System.Globalization.CultureInfo.InvariantCulture);
                }
            }
            if (length < 0 || length > Limit) throw Failed();
            byte[] response = new byte[length];
            int read = 0;
            while (read < length)
            {
                Timeout(stream, deadline, timeout);
                int count = stream.Read(response, read, length - read);
                if (count <= 0) throw Failed();
                read += count;
            }
            return response;
        }
        catch (Exception) { throw Failed(); }
    }
    private static void Timeout(System.Net.Sockets.NetworkStream stream, System.Diagnostics.Stopwatch deadline, int timeout)
    {
        long remaining = timeout - deadline.ElapsedMilliseconds;
        if (remaining <= 0) throw Failed();
        stream.ReadTimeout = (int)remaining;
    }
}

static class YonGateway
{
    private const double AnswerMarginMilliseconds = 1000;
    internal static string Call(string key, string arguments)
    {
        byte[] body = Encoding.UTF8.GetBytes("{\"operation\":\"gateway\",\"key\":" + JsonSerializer.Serialize(key) + ",\"arguments\":" + arguments + "}");
        if (body.Length > YonManagedResources.Limit) return Failure("invalid");
        double timeLeft = Yon.TimeLeftMilliseconds();
        if (timeLeft <= 0) return Failure("timeout");
        byte[] answer;
        try
        {
            answer = YonManagedResources.Exchange(body, (long)(timeLeft + AnswerMarginMilliseconds));
        }
        catch (Exception)
        {
            if (Yon.TimeLeftMilliseconds() <= 0) return Failure("timeout");
            return Failure("adapter");
        }
        if (!IsAnswer(answer)) return Failure("adapter");
        return Encoding.UTF8.GetString(answer);
    }
    private static bool IsAnswer(byte[] answer)
    {
        try
        {
            using var document = JsonDocument.Parse(answer);
            var root = document.RootElement;
            if (root.ValueKind != JsonValueKind.Object) return false;
            if (root.TryGetProperty("result", out _)) return true;
            return root.TryGetProperty("error", out var kind) && kind.ValueKind == JsonValueKind.String;
        }
        catch (Exception) { return false; }
    }
    private static string Failure(string kind) => "{\"error\":\"" + kind + "\"}";
}

static class Json
{
    public static string Write(object value) => JsonSerializer.Serialize(value);
    public static string Stringify(object value) => JsonSerializer.Serialize(value);

    public static JsonNode Parse(string source) => JsonNode.Parse(source);
}

sealed class __YonFrame
{
    private readonly JsonObject raw;

    internal __YonFrame(JsonNode parsed)
    {
        raw = parsed as JsonObject ?? new JsonObject();
    }

    public string Method() => Text(raw["method"]);

    public string Route() => Text(raw["route"]);

    public string Parameter(string name) =>
        raw["parameters"] is JsonObject parameters ? Text(parameters[name]) : "";

    public string Body() =>
        raw["body"] is JsonObject body ? Text(body["data"]) : "";

    public JsonNode Parsed() => Json.Parse(Body());

    public JsonObject Raw() => raw;

    public JsonArray Arguments() => raw["arguments"] as JsonArray ?? new JsonArray();

    public string Target() => raw["target"] is JsonNode target ? Text(target) : Route();

    public IEnumerable<KeyValuePair<string, string>> Headers()
    {
        if (raw["headers"] is not JsonObject headers) yield break;
        foreach (var pair in headers)
        {
            if (pair.Value is not JsonArray values) continue;
            foreach (var value in values) yield return new(pair.Key, Text(value));
        }
    }

    private static string Text(JsonNode node) =>
        node is null ? "" : node.GetValue<object>()?.ToString() ?? node.ToJsonString();
}

static class YonModels
{
    internal static readonly JsonSerializerOptions Options = CreateOptions();

    static JsonSerializerOptions CreateOptions()
    {
        var options = new JsonSerializerOptions(JsonSerializerDefaults.Web);
        options.Converters.Add(new System.Text.Json.Serialization.JsonStringEnumConverter());
        return options;
    }

    internal static T Argument<T>(JsonArray values, int index)
    {
        var node = index < values.Count ? values[index] : null;
        return node is null ? default : node.Deserialize<T>(Options);
    }

    internal static object Answer(object value)
    {
        if (value is null || value is __YonAnswer || value is HttpResponseMessage || value is IEnumerator || value is IAsyncEnumerable<object>)
        {
            return value;
        }
        return JsonSerializer.SerializeToNode(value, value.GetType(), Options);
    }
}

sealed class __YonAnswer
{
    private int status;
    private readonly Dictionary<string, List<string>> headers = new();
    private readonly string body;

    private __YonAnswer(int status, string body)
    {
        this.status = status;
        this.body = body;
    }

    public static __YonAnswer Json(string body)
    {
        var response = new __YonAnswer(200, body);
        response.headers["content-type"] = new List<string> { "application/json" };
        return response;
    }

    public static __YonAnswer Empty(int status) => new(status, "");

    public __YonAnswer Status(int value)
    {
        status = value;
        return this;
    }

    internal __YonAnswer Headers(Dictionary<string, List<string>> replacement)
    {
        headers.Clear();
        foreach (var pair in replacement) headers[pair.Key] = pair.Value;
        return this;
    }

    public __YonAnswer Header(string name, string value)
    {
        if (!headers.TryGetValue(name, out var values))
        {
            values = new List<string>();
            headers[name] = values;
        }
        values.Add(value);
        return this;
    }

    internal HttpResponseMessage Message()
    {
        var message = new HttpResponseMessage((System.Net.HttpStatusCode)status)
        {
            Content = new StringContent(body),
        };
        message.Content.Headers.Clear();
        foreach (var pair in headers)
        {
            if (message.Headers.TryAddWithoutValidation(pair.Key, pair.Value)) continue;
            message.Content.Headers.TryAddWithoutValidation(pair.Key, pair.Value);
        }
        return message;
    }

    internal string Envelope()
    {
        var envelope = new JsonObject
        {
            ["status"] = status,
            ["body"] = body,
        };
        var written = new JsonObject();
        foreach (var pair in headers)
        {
            var list = new JsonArray();
            foreach (var value in pair.Value) list.Add(value);
            written[pair.Key] = list;
        }
        envelope["headers"] = written;
        return envelope.ToJsonString();
    }
}

static class YonHttp
{
    internal static HttpRequestMessage Request(__YonFrame frame)
    {
        var host = "localhost";
        foreach (var header in frame.Headers())
        {
            if (header.Key.Equals("host", StringComparison.OrdinalIgnoreCase)) host = header.Value;
        }
        var request = new HttpRequestMessage(new HttpMethod(frame.Method()), new Uri("http://" + host + frame.Target()));
        var body = frame.Body();
        if (body.Length > 0) request.Content = new ByteArrayContent(Encoding.UTF8.GetBytes(body));
        foreach (var header in frame.Headers())
        {
            if (request.Headers.TryAddWithoutValidation(header.Key, header.Value)) continue;
            request.Content?.Headers.TryAddWithoutValidation(header.Key, header.Value);
        }
        return request;
    }

    internal static __YonAnswer Answer(HttpResponseMessage response)
    {
        var headers = new Dictionary<string, List<string>>();
        foreach (var header in response.Headers) headers[header.Key.ToLowerInvariant()] = new List<string>(header.Value);
        var body = "";
        if (response.Content is not null)
        {
            foreach (var header in response.Content.Headers) headers[header.Key.ToLowerInvariant()] = new List<string>(header.Value);
            body = response.Content.ReadAsStringAsync().GetAwaiter().GetResult();
        }
        return __YonAnswer.Json(body).Headers(headers).Status((int)response.StatusCode);
    }

    internal static HttpResponseMessage Response(__YonAnswer answer) => answer.Message();

    internal static JsonObject Message(HttpRequestMessage request)
    {
        var headers = new JsonObject();
        foreach (var header in request.Headers) headers[header.Key.ToLowerInvariant()] = Values(header.Value);
        var body = "";
        if (request.Content is not null)
        {
            foreach (var header in request.Content.Headers) headers[header.Key.ToLowerInvariant()] = Values(header.Value);
            body = request.Content.ReadAsStringAsync().GetAwaiter().GetResult();
        }
        return new JsonObject
        {
            ["route"] = request.RequestUri?.PathAndQuery ?? "/",
            ["method"] = request.Method.Method,
            ["headers"] = headers,
            ["body"] = new JsonObject { ["encoding"] = "utf8", ["data"] = body },
        };
    }

    static JsonArray Values(IEnumerable<string> values)
    {
        var list = new JsonArray();
        foreach (var value in values) list.Add(value);
        return list;
    }
}

static class YonStream
{
    const int MaxFrameBytes = 16 * 1024 * 1024;
    const string FsharpSequence = "Microsoft.FSharp.Core.CompilerServices.GeneratedSequenceBase`1";

    public static bool Generated(Type type) =>
        type.IsDefined(typeof(System.Runtime.CompilerServices.CompilerGeneratedAttribute), false) ||
        (type.BaseType is { IsGenericType: true } parent && parent.GetGenericTypeDefinition().FullName == FsharpSequence);

    static readonly Stream Output = Console.OpenStandardOutput();

    public static bool Event(string requestId, object value)
    {
        var frame = new JsonObject
        {
            ["protocol_version"] = 1,
            ["kind"] = "event",
            ["request_id"] = requestId,
            ["body"] = new JsonObject
            {
                ["encoding"] = "utf8",
                ["data"] = value is null ? "null" : JsonSerializer.Serialize(value, value.GetType(), YonModels.Options),
            },
        };
        return Write(requestId, frame);
    }

    public static void Failure(string requestId)
    {
        Write(requestId, Envelope(requestId, "TY2204", "The handler stream failed."));
    }

    static JsonObject Envelope(string requestId, string code, string message) =>
        new()
        {
            ["protocol_version"] = 1,
            ["kind"] = "response",
            ["request_id"] = requestId,
            ["status"] = 500,
            ["headers"] = new JsonObject(),
            ["error"] = new JsonObject
            {
                ["code"] = code,
                ["message"] = message,
                ["retryable"] = false,
            },
        };

    static bool Write(string requestId, JsonObject frame)
    {
        var payload = Encoding.UTF8.GetBytes(frame.ToJsonString());
        if (payload.Length > MaxFrameBytes)
        {
            var refusal = Encoding.UTF8.GetBytes(
                Envelope(requestId, "TY2203", "Streamed event exceeds the protocol frame limit.")
                    .ToJsonString());
            Emit(refusal);
            return false;
        }
        Emit(payload);
        return true;
    }

    static void Emit(byte[] payload)
    {
        var prefix = new byte[4];
        BinaryPrimitives.WriteUInt32BigEndian(prefix, (uint)payload.Length);
        Output.Write(prefix, 0, prefix.Length);
        Output.Write(payload, 0, payload.Length);
        Output.Flush();
    }
}

interface IYonSocketEndpoint
{
    void Open(SocketSession session);
    void Message(SocketSession session, string data);
    void Close(SocketSession session);
    void Error(SocketSession session, Exception error);
}

public sealed class SocketPeer
{
    private readonly SocketSession session;

    internal SocketPeer(SocketSession session, string id)
    {
        this.session = session;
        Id = id;
    }

    public string Id { get; }

    public void Send(object message) => session.Send(message, Id);
}

public sealed class SocketSession
{
    private readonly Queue<string> inbox = new();
    private bool closed;
    private List<string> sessions;

    internal SocketSession(string id)
    {
        Id = id;
    }

    public string Id { get; }

    public void Send(object message) => Send(message, null);

    public void Close()
    {
        closed = true;
        Write(new JsonObject { ["kind"] = "close" });
    }

    public List<SocketPeer> OpenSessions()
    {
        sessions = null;
        Write(new JsonObject { ["kind"] = "sessions" });
        while (sessions is null && !closed) Receive();
        var peers = new List<SocketPeer>();
        foreach (var id in sessions ?? new List<string>()) peers.Add(new SocketPeer(this, id));
        return peers;
    }

    internal void Send(object message, string to)
    {
        var data = message is null ? "null" : JsonSerializer.Serialize(message, message.GetType(), YonModels.Options);
        var frame = new JsonObject
        {
            ["kind"] = "event",
            ["body"] = new JsonObject { ["encoding"] = "utf8", ["data"] = data },
        };
        if (to is not null) frame["to"] = to;
        Write(frame);
    }

    internal string Next()
    {
        while (inbox.Count == 0 && !closed) Receive();
        return inbox.Count == 0 ? null : inbox.Dequeue();
    }

    private void Receive()
    {
        var raw = YonSocket.ReadFrame();
        if (raw is null || JsonNode.Parse(raw) is not JsonObject frame)
        {
            closed = true;
            return;
        }
        if (frame["request_id"]?.ToString() != Id) Environment.Exit(70);
        switch (frame["kind"]?.ToString())
        {
            case "message":
                inbox.Enqueue(frame["body"]?["data"]?.ToString() ?? "null");
                break;
            case "sessions":
                sessions = new List<string>();
                if (frame["sessions"] is JsonArray list)
                {
                    foreach (var id in list) sessions.Add(id?.ToString() ?? "");
                }
                break;
            default:
                closed = true;
                break;
        }
    }

    private void Write(JsonObject frame)
    {
        frame["protocol_version"] = 1;
        frame["request_id"] = Id;
        YonSocket.WriteFrame(frame.ToJsonString());
    }
}

static class YonSocket
{
    const int MaxFrameBytes = 16 * 1024 * 1024;
    static readonly Stream Input = Console.OpenStandardInput();
    static readonly Stream Output = Console.OpenStandardOutput();

    internal static bool Framed() => Environment.GetEnvironmentVariable("YON_SOCKET_FRAMES") == "1";

    internal static string ReadFrame()
    {
        var prefix = new byte[4];
        if (!ReadAll(prefix)) return null;
        var length = BinaryPrimitives.ReadUInt32BigEndian(prefix);
        if (length > MaxFrameBytes) Environment.Exit(70);
        var payload = new byte[length];
        return ReadAll(payload) ? Encoding.UTF8.GetString(payload) : null;
    }

    static bool ReadAll(byte[] buffer)
    {
        try
        {
            Input.ReadExactly(buffer, 0, buffer.Length);
            return true;
        }
        catch (EndOfStreamException)
        {
            return false;
        }
    }

    internal static void WriteFrame(string text)
    {
        var payload = Encoding.UTF8.GetBytes(text);
        var prefix = new byte[4];
        BinaryPrimitives.WriteUInt32BigEndian(prefix, (uint)payload.Length);
        Output.Write(prefix, 0, prefix.Length);
        Output.Write(payload, 0, payload.Length);
        Output.Flush();
    }

    internal static T Read<T>(string data) => JsonSerializer.Deserialize<T>(data, YonModels.Options);

    internal static void Call(Action action) => action();

    internal static void Call(Func<Task> action) => action().GetAwaiter().GetResult();

    internal static object Run(IYonSocketEndpoint endpoint, __YonFrame request)
    {
        var session = new SocketSession(request.Raw()["request_id"]?.ToString() ?? "");
        Attempt(endpoint, session, () => endpoint.Open(session));
        while (session.Next() is string message)
        {
            Attempt(endpoint, session, () => endpoint.Message(session, message));
        }
        try
        {
            Yon.RenewDeadline();
            endpoint.Close(session);
        }
        catch (Exception)
        {
        }
        Environment.Exit(0);
        return null;
    }

    static void Attempt(IYonSocketEndpoint endpoint, SocketSession session, Action action)
    {
        try
        {
            Yon.RenewDeadline();
            action();
        }
        catch (Exception failure)
        {
            var line = new JsonObject
            {
                ["event"] = "handler.socket_failed",
                ["request_id"] = session.Id,
                ["message"] = failure.Message,
            };
            Console.Error.WriteLine(line.ToJsonString());
            try
            {
                Yon.RenewDeadline();
                endpoint.Error(session, failure);
            }
            catch (Exception)
            {
            }
        }
    }
}

static class Yon
{
    const int MaxRelayStdoutBytes = 16 * 1024 * 1024;
    const int MaxRelayStderrBytes = 64 * 1024;
    const double DefaultDeadlineMilliseconds = 30_000;
    const string FailureMessage = "Component invocation failed.";
    static readonly long startTimestamp = ProcessStartTimestamp();
    static Dictionary<string, string> relayEnvironment;
    static double? deadlineMilliseconds;
    static string requestId = "unknown";

    sealed class RelayOutput
    {
        public byte[] Bytes { get; init; } = Array.Empty<byte>();
        public bool Overflow { get; init; }
    }

    static async System.Threading.Tasks.Task<RelayOutput> Drain(
        System.IO.Stream pipe,
        int limit)
    {
        using var kept = new System.IO.MemoryStream(Math.Min(limit, 8192));
        var chunk = new byte[8192];
        var overflow = false;
        while (true)
        {
            int count;
            try
            {
                count = await pipe.ReadAsync(chunk.AsMemory());
            }
            catch (Exception)
            {
                overflow = true;
                break;
            }
            if (count == 0) break;
            var remaining = Math.Max(0, limit - (int)kept.Length);
            await kept.WriteAsync(chunk.AsMemory(0, Math.Min(count, remaining)));
            overflow |= count > remaining;
        }
        return new RelayOutput { Bytes = kept.ToArray(), Overflow = overflow };
    }

    static long ProcessStartTimestamp()
    {
        var now = System.Diagnostics.Stopwatch.GetTimestamp();
        try
        {
            var sinceStart = DateTime.Now - System.Diagnostics.Process.GetCurrentProcess().StartTime;
            var ticks = (long)(Math.Max(0, sinceStart.TotalSeconds) * System.Diagnostics.Stopwatch.Frequency);
            return now - ticks;
        }
        catch (Exception)
        {
            return now;
        }
    }

    static void CaptureStart()
    {
        if (relayEnvironment is not null) return;
        relayEnvironment = RelayEnvironment();
    }

    static Dictionary<string, string> RelayEnvironment()
    {
        var kept = new Dictionary<string, string>();
        foreach (DictionaryEntry entry in Environment.GetEnvironmentVariables())
        {
            var name = (string)entry.Key;
            var upper = name.ToUpperInvariant();
            if (upper == "YON_APPLICATION_JSON" || upper.StartsWith("YON_RESOURCE_", StringComparison.Ordinal)) continue;
            kept[name] = (string)entry.Value;
        }
        return kept;
    }

    static void CaptureInvocation(__YonFrame request)
    {
        if (deadlineMilliseconds is not null) return;
        if (request.Raw()["request_id"] is JsonValue id && id.TryGetValue<string>(out var text)) requestId = text;
        deadlineMilliseconds = InvocationDeadlineMilliseconds(request);
    }

    static double InvocationDeadlineMilliseconds(__YonFrame request)
    {
        if (request.Raw()["deadline_ms"] is not JsonValue requested) return DefaultDeadlineMilliseconds;
        if (!requested.TryGetValue<double>(out var milliseconds)) return DefaultDeadlineMilliseconds;
        return milliseconds;
    }

    internal static void RenewDeadline()
    {
        deadlineMilliseconds = System.Diagnostics.Stopwatch.GetElapsedTime(startTimestamp).TotalMilliseconds
            + DefaultDeadlineMilliseconds;
    }

    internal static double TimeLeftMilliseconds()
    {
        if (deadlineMilliseconds is not double deadline) return 0;
        return deadline - System.Diagnostics.Stopwatch.GetElapsedTime(startTimestamp).TotalMilliseconds;
    }

    public static HttpResponseMessage Relay(IReadOnlyList<string> command, HttpRequestMessage request) =>
        YonHttp.Response(RelayAnswer(command, request));

    static __YonAnswer RelayAnswer(IReadOnlyList<string> command, HttpRequestMessage request)
    {
        if (command.Count == 0) return RelayFailed("start");
        if (TimeLeftMilliseconds() <= 0) return RelayFailed("timeout");
        string payload;
        try
        {
            payload = YonHttp.Message(request).ToJsonString();
        }
        catch (Exception)
        {
            return RelayFailed("protocol");
        }
        System.Diagnostics.Process child;
        try
        {
            child = StartChild(command);
        }
        catch (Exception)
        {
            return RelayFailed("start");
        }
        if (child is null) return RelayFailed("start");
        using (child)
        {
            try
            {
                return AwaitChild(child, payload);
            }
            catch (Exception)
            {
                return RelayFailed("exit");
            }
        }
    }

    static System.Diagnostics.Process StartChild(IReadOnlyList<string> command)
    {
        var start = new System.Diagnostics.ProcessStartInfo(command[0])
        {
            RedirectStandardInput = true,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            UseShellExecute = false,
        };
        for (var index = 1; index < command.Count; index += 1)
        {
            start.ArgumentList.Add(command[index]);
        }
        start.Environment.Clear();
        foreach (var pair in relayEnvironment)
        {
            start.Environment[pair.Key] = pair.Value;
        }
        return System.Diagnostics.Process.Start(start);
    }

    static __YonAnswer AwaitChild(System.Diagnostics.Process child, string payload)
    {
        var stdoutTask = Drain(child.StandardOutput.BaseStream, MaxRelayStdoutBytes);
        var stderrTask = Drain(child.StandardError.BaseStream, MaxRelayStderrBytes);
        _ = Task.Run(() => SendRequest(child.StandardInput, payload));
        if (!FinishesInTime(child.WaitForExitAsync())) return StopChild(child);
        if (!FinishesInTime(Task.WhenAll(stdoutTask, stderrTask))) return StopChild(child);
        return RelayResult(child.ExitCode, stdoutTask.Result, stderrTask.Result);
    }

    static void SendRequest(StreamWriter stdin, string payload)
    {
        try
        {
            stdin.Write(payload);
            stdin.Close();
        }
        catch (Exception)
        {
        }
    }

    static bool FinishesInTime(Task work)
    {
        var milliseconds = (int)Math.Clamp(TimeLeftMilliseconds(), 0, int.MaxValue);
        return work.Wait(milliseconds);
    }

    static __YonAnswer StopChild(System.Diagnostics.Process child)
    {
        try
        {
            child.Kill(entireProcessTree: true);
        }
        catch (Exception)
        {
        }
        return RelayFailed("timeout");
    }

    static __YonAnswer RelayResult(int exitCode, RelayOutput stdout, RelayOutput stderr)
    {
        if (stdout.Overflow || stderr.Overflow) return RelayFailed("overflow");
        if (exitCode != 0) return RelayFailed("exit");
        try
        {
            if (JsonNode.Parse(stdout.Bytes) is not JsonObject envelope)
            {
                return RelayFailed("protocol");
            }
            return ResponseFromEnvelope(envelope);
        }
        catch (Exception)
        {
            return RelayFailed("protocol");
        }
    }

    static __YonAnswer ResponseFromEnvelope(JsonObject envelope)
    {
        var response = __YonAnswer.Json(envelope["body"]?.GetValue<string>() ?? "");
        if (envelope["status"] is JsonValue status)
        {
            response = response.Status(status.GetValue<int>());
        }
        if (envelope["headers"] is not JsonObject written || written.Count == 0) return response;
        response = response.Headers(new Dictionary<string, List<string>>());
        foreach (var pair in written)
        {
            var values = pair.Value is JsonArray list
                ? list
                : new JsonArray(pair.Value?.DeepClone());
            foreach (var value in values)
            {
                response = response.Header(pair.Key, value?.GetValue<string>() ?? "");
            }
        }
        return response;
    }

    static __YonAnswer RelayFailed(string category)
    {
        var failure = new JsonObject
        {
            ["event"] = "handler.relay_failed",
            ["request_id"] = requestId,
            ["category"] = category,
        };
        Console.Error.Write(failure.ToJsonString() + "\n");
        Console.Error.Flush();
        return __YonAnswer.Json(JsonSerializer.Serialize(new { error = FailureMessage })).Status(502);
    }

    static Type entryType;

    static MethodInfo EntryMethod(string name, Type[] parameters) =>
        entryType.GetMethod(
            name,
            BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Static,
            null,
            parameters,
            null);

    static Task Main() => Run(typeof(__YON_HANDLER__));

    internal static async Task Run(Type entry)
    {
        entryType = entry;
        CaptureStart();
        var raw = YonSocket.Framed() ? YonSocket.ReadFrame() : Console.In.ReadToEnd();
        var parsed = string.IsNullOrWhiteSpace(raw) ? new JsonObject() : JsonNode.Parse(raw);
        var request = new __YonFrame(parsed);
        CaptureInvocation(request);

        var phase = request.Raw()["operation"]?.ToString() switch
        {
            "middleware.before" => "before",
            "middleware.after" => "after",
            "event.deliver" => "__YonDeliver",
            _ => null,
        };
        var entryName = phase ?? request.Method();
        var missingStatus = phase is null ? 405 : phase == "__YonDeliver" ? 404 : 204;
        object result;
        var method = EntryMethod(entryName, new[] { typeof(__YonFrame) })
            ?? EntryMethod(entryName, new[] { typeof(HttpRequestMessage) })
            ?? EntryMethod(entryName, Type.EmptyTypes);
        if (method is null)
        {
            result = __YonAnswer.Empty(missingStatus);
        }
        else
        {
            try
            {
                var parameters = method.GetParameters();
                object[] arguments = null;
                if (parameters.Length == 1 && parameters[0].ParameterType == typeof(__YonFrame)) arguments = new object[] { request };
                if (parameters.Length == 1 && parameters[0].ParameterType == typeof(HttpRequestMessage)) arguments = new object[] { YonHttp.Request(request) };
                result = method.Invoke(null, arguments);
            }
            catch (TargetInvocationException)
            {
                Environment.ExitCode = 1;
                return;
            }
        }

        var id = request.Raw()["request_id"]?.ToString() ?? "";
        if (result is IAsyncEnumerable<object> asynchronous)
        {
            try
            {
                await foreach (var value in asynchronous)
                {
                    if (!YonStream.Event(id, value)) return;
                }
            }
            catch (Exception)
            {
                YonStream.Failure(id);
            }
            return;
        }
        if (result is IEnumerable sequence && result is IEnumerator && YonStream.Generated(result.GetType()))
        {
            try
            {
                foreach (var value in sequence)
                {
                    if (!YonStream.Event(id, value)) return;
                }
            }
            catch (Exception)
            {
                YonStream.Failure(id);
            }
            return;
        }
        var response = result switch
        {
            __YonAnswer explicitResponse => explicitResponse,
            HttpResponseMessage message => YonHttp.Answer(message),
            null when phase is not null => __YonAnswer.Empty(204),
            _ => __YonAnswer.Json(JsonSerializer.Serialize(result)),
        };
        Console.Out.Write(response.Envelope());
    }
}
