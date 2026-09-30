public sealed class BehaviorError : System.Exception {
    public string Code { get; }
    public string Path { get; }
    public BehaviorError(string code, string path = null) : base("Behavior operation failed: " + code) { Code = code; Path = path != null && System.Text.Encoding.UTF8.GetByteCount(path) <= 128 ? path : null; }
    internal BehaviorError(string code, string path, string message) : base(message) { Code = code; Path = path; }
}
public sealed class BehaviorJSONConverter : System.Text.Json.Serialization.JsonConverter<BehaviorJSON> {
    public override BehaviorJSON Read(ref System.Text.Json.Utf8JsonReader reader, System.Type type, System.Text.Json.JsonSerializerOptions options) {
        using var document = System.Text.Json.JsonDocument.ParseValue(ref reader);
        return BehaviorJSON.Parse(document.RootElement.GetRawText());
    }
    public override void Write(System.Text.Json.Utf8JsonWriter writer, BehaviorJSON value, System.Text.Json.JsonSerializerOptions options) => writer.WriteRawValue(value.ToString());
}
[System.Text.Json.Serialization.JsonConverter(typeof(BehaviorJSONConverter))]
public sealed class BehaviorJSON {
    internal System.Text.Json.JsonElement Value { get; }
    private BehaviorJSON(System.Text.Json.JsonElement value) { Value = value.Clone(); }
    public static BehaviorJSON Parse(string source) {
        if (System.Text.Encoding.UTF8.GetByteCount(source) > 1048576) throw new BehaviorError("JSON_LIMIT");
        try {
            using var document = System.Text.Json.JsonDocument.Parse(source, new System.Text.Json.JsonDocumentOptions { MaxDepth = 32 });
            Validate(document.RootElement);
            return new BehaviorJSON(document.RootElement);
        } catch (BehaviorError) { throw; } catch { throw new BehaviorError("JSON_INVALID"); }
    }
    private static void Validate(System.Text.Json.JsonElement value) {
        if (value.ValueKind == System.Text.Json.JsonValueKind.Number && (!value.TryGetDouble(out var number) || !double.IsFinite(number) || System.Math.Abs(number) > 9007199254740991d)) throw new BehaviorError("JSON_NUMBER");
        if (value.ValueKind == System.Text.Json.JsonValueKind.Array) foreach (var child in value.EnumerateArray()) Validate(child);
        if (value.ValueKind == System.Text.Json.JsonValueKind.Object) {
            var seen = new System.Collections.Generic.HashSet<string>(System.StringComparer.Ordinal);
            foreach (var property in value.EnumerateObject()) {
                if (!seen.Add(property.Name)) throw new BehaviorError("JSON_DUPLICATE");
                Validate(property.Value);
            }
        }
    }
    public static BehaviorJSON From(object value) {
        if (value == null) return Parse("null");
        if (value is BehaviorJSON json) return json;
        if (value is string text) return Parse(Quote(text));
        if (value is bool boolean) return Parse(boolean ? "true" : "false");
        if (value is sbyte || value is byte || value is short || value is ushort || value is int || value is uint || value is long || value is ulong || value is float || value is double || value is decimal) return Parse(System.Convert.ToString(value, System.Globalization.CultureInfo.InvariantCulture));
        throw new BehaviorError("JSON_TYPE");
    }
    public static BehaviorJSON Object(System.Collections.Generic.IDictionary<string, BehaviorJSON> values) {
        var sorted = new System.Collections.Generic.SortedDictionary<string, BehaviorJSON>(values, System.StringComparer.Ordinal);
        return Write(writer => {
            writer.WriteStartObject();
            foreach (var pair in sorted) {
                writer.WritePropertyName(pair.Key);
                pair.Value.Value.WriteTo(writer);
            }
            writer.WriteEndObject();
        });
    }
    public static BehaviorJSON Array(params BehaviorJSON[] values) {
        return Write(writer => {
            writer.WriteStartArray();
            foreach (var value in values) value.Value.WriteTo(writer);
            writer.WriteEndArray();
        });
    }
    private static BehaviorJSON Write(System.Action<System.Text.Json.Utf8JsonWriter> write) {
        using var stream = new System.IO.MemoryStream();
        using var writer = new System.Text.Json.Utf8JsonWriter(stream);
        write(writer);
        writer.Flush();
        return Parse(System.Text.Encoding.UTF8.GetString(stream.ToArray()));
    }
    internal static string Quote(string value) {
        return "\"" + System.Text.Json.JsonEncodedText.Encode(value).ToString() + "\"";
    }
    public BehaviorJSON Get(string key) {
        if (Value.ValueKind != System.Text.Json.JsonValueKind.Object || !Value.TryGetProperty(key, out var value)) return Parse("null");
        return new BehaviorJSON(value);
    }
    public bool Has(string key) => Value.ValueKind == System.Text.Json.JsonValueKind.Object && Value.TryGetProperty(key, out _);
    public bool IsNull => Value.ValueKind == System.Text.Json.JsonValueKind.Null;
    public string Text { get { if (Value.ValueKind != System.Text.Json.JsonValueKind.String) throw new BehaviorError("JSON_STRING"); return Value.GetString(); } }
    public long Integer { get { if (Value.ValueKind != System.Text.Json.JsonValueKind.Number || !Value.TryGetInt64(out var result)) throw new BehaviorError("JSON_INTEGER"); return result; } }
    public double Number { get { if (Value.ValueKind != System.Text.Json.JsonValueKind.Number || !Value.TryGetDouble(out var result)) throw new BehaviorError("JSON_NUMBER"); return result; } }
    public bool Boolean { get { if (Value.ValueKind != System.Text.Json.JsonValueKind.True && Value.ValueKind != System.Text.Json.JsonValueKind.False) throw new BehaviorError("JSON_BOOLEAN"); return Value.GetBoolean(); } }
    public BehaviorJSON[] Items {
        get {
            if (Value.ValueKind != System.Text.Json.JsonValueKind.Array) throw new BehaviorError("JSON_ARRAY");
            var result = new System.Collections.Generic.List<BehaviorJSON>();
            foreach (var item in Value.EnumerateArray()) result.Add(new BehaviorJSON(item));
            return result.ToArray();
        }
    }
    public System.Collections.Generic.Dictionary<string, BehaviorJSON> Members {
        get {
            if (Value.ValueKind != System.Text.Json.JsonValueKind.Object) throw new BehaviorError("JSON_OBJECT");
            var result = new System.Collections.Generic.Dictionary<string, BehaviorJSON>(System.StringComparer.Ordinal);
            foreach (var property in Value.EnumerateObject()) result.Add(property.Name, new BehaviorJSON(property.Value));
            return result;
        }
    }
    public override string ToString() => Canonical(Value);
    private static string Canonical(System.Text.Json.JsonElement value) {
        if (value.ValueKind == System.Text.Json.JsonValueKind.Object) {
            var members = new System.Collections.Generic.SortedDictionary<string, string>(System.StringComparer.Ordinal);
            foreach (var property in value.EnumerateObject()) members.Add(property.Name, Canonical(property.Value));
            var entries = new System.Collections.Generic.List<string>();
            foreach (var pair in members) entries.Add(Quote(pair.Key) + ":" + pair.Value);
            return "{" + string.Join(",", entries) + "}";
        }
        if (value.ValueKind == System.Text.Json.JsonValueKind.Array) {
            var entries = new System.Collections.Generic.List<string>();
            foreach (var item in value.EnumerateArray()) entries.Add(Canonical(item));
            return "[" + string.Join(",", entries) + "]";
        }
        if (value.ValueKind == System.Text.Json.JsonValueKind.Number) { double number = value.GetDouble(); return number == 0 ? "0" : number.ToString("R", System.Globalization.CultureInfo.InvariantCulture); }
        if (value.ValueKind == System.Text.Json.JsonValueKind.String) return Quote(value.GetString());
        return value.GetRawText();
    }
    internal void Bound(int maximum) { if (System.Text.Encoding.UTF8.GetByteCount(ToString()) > maximum) throw new BehaviorError("JSON_LIMIT"); }
    internal void Keys(params string[] allowed) { foreach (var key in Members.Keys) if (System.Array.IndexOf(allowed, key) < 0) throw new BehaviorError("UNKNOWN_FIELD"); }
}
public sealed class BehaviorSignal : System.IDisposable {
    private readonly System.Threading.CancellationTokenSource source = new System.Threading.CancellationTokenSource();
    public System.Threading.CancellationToken Token => source.Token;
    public bool Aborted => source.IsCancellationRequested;
    public void Abort() => source.Cancel();
    public void Dispose() { source.Cancel(); }
}
public sealed class BehaviorExecution {
    public string Environment { get; }
    public string Partition { get; }
    public BehaviorSignal Signal { get; }
    public long Deadline { get; }
    internal BehaviorExecution(string partition, BehaviorSignal signal, long deadline) { Environment = Behaviors.Environment; Partition = partition; Signal = signal; Deadline = deadline; }
    public void Check() { if (Signal.Aborted) throw new BehaviorError("CANCELLED"); if (Behaviors.Now >= Deadline) throw new BehaviorError("DEADLINE"); }
}
public interface BehaviorCacheAdapter {
    System.Threading.Tasks.Task<BehaviorJSON> get(string key, BehaviorExecution execution);
    System.Threading.Tasks.Task set(string key, BehaviorJSON value, BehaviorJSON policy, long expiresAt, BehaviorExecution execution);
    System.Threading.Tasks.Task delete(string key, BehaviorExecution execution);
}
public interface BehaviorHTTPAdapter { System.Threading.Tasks.Task<BehaviorJSON> exchange(BehaviorJSON request, BehaviorExecution execution); }
public interface BehaviorEntityAdapter { System.Threading.Tasks.Task<BehaviorJSON> execute(BehaviorJSON command, BehaviorExecution execution); }
public static class BehaviorStorage {
    public static System.Func<string, string> Host { get; set; }
    internal static bool Available => Behaviors.Environment == "tac" && Host != null;
    internal static BehaviorJSON Ask(string operation, System.Collections.Generic.Dictionary<string, BehaviorJSON> request) {
        request["op"] = BehaviorJSON.From(operation);
        var answer = BehaviorJSON.Parse(Host(BehaviorJSON.Object(request).ToString()));
        if (answer.Has("error")) throw new BehaviorError("STORAGE", null, answer.Get("error").Text);
        return answer.Get("value");
    }
    internal static BehaviorCacheAdapter Cache(string name) => Available ? new BehaviorHostCache(name) : null;
    internal static BehaviorEntityAdapter Entities(string name) => Available && name == "indexeddb" ? new BehaviorHostEntities() : null;
}
internal sealed class BehaviorHostCache : BehaviorCacheAdapter {
    private readonly string name;
    internal BehaviorHostCache(string name) { this.name = name; }
    public System.Threading.Tasks.Task<BehaviorJSON> get(string key, BehaviorExecution execution) =>
        System.Threading.Tasks.Task.FromResult(BehaviorStorage.Ask("cache.get", new System.Collections.Generic.Dictionary<string, BehaviorJSON> { ["cache"] = BehaviorJSON.From(name), ["key"] = BehaviorJSON.From(key) }));
    public System.Threading.Tasks.Task set(string key, BehaviorJSON value, BehaviorJSON policy, long expiresAt, BehaviorExecution execution) {
        BehaviorStorage.Ask("cache.set", new System.Collections.Generic.Dictionary<string, BehaviorJSON> { ["cache"] = BehaviorJSON.From(name), ["key"] = BehaviorJSON.From(key), ["value"] = value, ["expiresAt"] = BehaviorJSON.From(expiresAt), ["maxEntries"] = policy.Get("maxEntries") });
        return System.Threading.Tasks.Task.CompletedTask;
    }
    public System.Threading.Tasks.Task delete(string key, BehaviorExecution execution) {
        BehaviorStorage.Ask("cache.delete", new System.Collections.Generic.Dictionary<string, BehaviorJSON> { ["cache"] = BehaviorJSON.From(name), ["key"] = BehaviorJSON.From(key) });
        return System.Threading.Tasks.Task.CompletedTask;
    }
}
internal sealed class BehaviorHostEntities : BehaviorEntityAdapter {
    public System.Threading.Tasks.Task<BehaviorJSON> execute(BehaviorJSON command, BehaviorExecution execution) =>
        System.Threading.Tasks.Task.FromResult(BehaviorStorage.Ask("entity", new System.Collections.Generic.Dictionary<string, BehaviorJSON> { ["command"] = command }));
}
public sealed class BehaviorAdapters {
    public readonly System.Collections.Generic.Dictionary<string, BehaviorCacheAdapter> caches = new System.Collections.Generic.Dictionary<string, BehaviorCacheAdapter>();
    public readonly System.Collections.Generic.Dictionary<string, BehaviorHTTPAdapter> http = new System.Collections.Generic.Dictionary<string, BehaviorHTTPAdapter>();
    public readonly System.Collections.Generic.Dictionary<string, BehaviorEntityAdapter> entities = new System.Collections.Generic.Dictionary<string, BehaviorEntityAdapter>();
    public System.Func<BehaviorJSON, BehaviorExecution, System.Threading.Tasks.Task<bool>> authorize;
    public System.Func<BehaviorJSON, BehaviorExecution, System.Threading.Tasks.Task<bool>> authorizeEntity;
}
public static class BehaviorPage {
    public static BehaviorJSON Normalize(BehaviorJSON input, BehaviorJSON policy) {
        if (input == null || input.IsNull) input = BehaviorJSON.Parse("{}");
        input.Keys("page", "size", "offset", "sort");
        long page = input.Has("page") ? input.Get("page").Integer : 0;
        long size = input.Has("size") ? input.Get("size").Integer : policy.Get("defaultSize").Integer;
        long maximum = policy.Get("maxSize").Integer;
        if (page < 0 || size < 1 || size > maximum || maximum > 1000 || page > 1000000 / size) throw new BehaviorError("PAGE_INVALID");
        var allowed = new System.Collections.Generic.HashSet<string>();
        foreach (var item in policy.Get("sort").Items) allowed.Add(item.Text);
        var seen = new System.Collections.Generic.HashSet<string>();
        var sort = new System.Collections.Generic.List<BehaviorJSON>();
        foreach (var item in input.Has("sort") ? input.Get("sort").Items : System.Array.Empty<BehaviorJSON>()) {
            var parts = item.Text.Split(':'); var field = parts[0]; var direction = parts.Length == 1 ? "asc" : parts[1];
            if (parts.Length > 2 || !allowed.Contains(field) || !seen.Add(field) || (direction != "asc" && direction != "desc")) throw new BehaviorError("PAGE_SORT");
            sort.Add(BehaviorJSON.From(field + ":" + direction));
        }
        return BehaviorJSON.Object(new System.Collections.Generic.Dictionary<string, BehaviorJSON> { ["page"] = BehaviorJSON.From(page), ["size"] = BehaviorJSON.From(size), ["offset"] = BehaviorJSON.From(page * size), ["sort"] = BehaviorJSON.Array(sort.ToArray()) });
    }
}
public static class Behaviors {
    internal const string Module = __TAC_BEHAVIOR_MODULE__;
    internal const string Environment = __TAC_BEHAVIOR_ENVIRONMENT__;
    internal static long Now => System.DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
    internal static BehaviorJSON Registration(string type) { var registration = BehaviorJSON.Parse(__TAC_BEHAVIOR_REGISTRY__).Get(type); if (registration.IsNull) throw new BehaviorError("UNREGISTERED"); return registration; }
    internal static BehaviorJSON Entity(System.Type entity) { __TAC_BEHAVIOR_ENTITIES__ throw new BehaviorError("ENTITY_UNKNOWN"); }
    private static BehaviorJSON TypePolicies(System.Type type) { __TAC_BEHAVIOR_TYPES__ return BehaviorJSON.Parse("{}"); }
    public static BehaviorJSON validate(System.Type type, BehaviorJSON value) {
        try { return ValidateSchema(TypePolicies(type).Get("validated"), value); }
        catch (BehaviorError error) when (error.Code == "validation") { throw; }
        catch { throw new BehaviorError("validation", "$.schema"); }
    }
    internal static BehaviorJSON ValidateSchema(BehaviorJSON schema, BehaviorJSON value) {
        try { if (value == null) throw new System.Exception(); value.Bound(1048576); }
        catch { throw new BehaviorError("validation", "$.json"); }
        if (value.Value.ValueKind != System.Text.Json.JsonValueKind.Object) throw new BehaviorError("validation", "$.object");
        try {
            schema.Keys("fields");
            foreach (var field in schema.Get("fields").Members) {
                if (!Matches(field.Key, @"\A[A-Za-z_$][A-Za-z0-9_$]{0,63}\z")) throw new BehaviorError("validation", "$.schema");
                var rules = field.Value; rules.Keys("notNull", "notBlank", "size", "min", "max"); var item = value.Get(field.Key);
                if (rules.Has("notNull") && rules.Get("notNull").Boolean && item.IsNull) throw new BehaviorError("validation", field.Key + ".notNull");
                if (rules.Has("notBlank") && rules.Get("notBlank").Boolean) {
                    if (item.Value.ValueKind != System.Text.Json.JsonValueKind.String || IsBlank(item.Text)) throw new BehaviorError("validation", field.Key + ".notBlank");
                }
                if (item.IsNull) continue;
                if (rules.Has("size")) {
                    var size = rules.Get("size"); size.Keys("min", "max");
                    int count = item.Value.ValueKind switch { System.Text.Json.JsonValueKind.String => System.Text.Encoding.UTF8.GetByteCount(item.Text), System.Text.Json.JsonValueKind.Array => item.Items.Length, System.Text.Json.JsonValueKind.Object => item.Members.Count, _ => -1 };
                    long minimum = size.Has("min") ? size.Get("min").Integer : 0, maximum = size.Has("max") ? size.Get("max").Integer : 1048576;
                    if (minimum < 0 || maximum < minimum) throw new BehaviorError("validation", "$.schema");
                    if (count < minimum || count > maximum) throw new BehaviorError("validation", field.Key + ".size");
                }
                foreach (var rule in new[] { "min", "max" }) if (rules.Has(rule)) {
                    if (item.Value.ValueKind != System.Text.Json.JsonValueKind.Number) throw new BehaviorError("validation", field.Key + "." + rule);
                    double bound = rules.Get(rule).Value.GetDouble(), number = item.Value.GetDouble();
                    if (!double.IsFinite(bound) || System.Math.Abs(bound) > 9007199254740991d) throw new BehaviorError("validation", "$.schema");
                    if ((rule == "min" && number < bound) || (rule == "max" && number > bound)) throw new BehaviorError("validation", field.Key + "." + rule);
                }
            }
            return value;
        } catch (BehaviorError error) when (error.Code == "validation") { throw; }
        catch { throw new BehaviorError("validation", "$.schema"); }
    }
    private static readonly string[] GatewayFailures = { "invalid", "timeout", "capacity", "adapter" };
    public static string GatewayArguments(BehaviorJSON[] values) {
        var arguments = new BehaviorJSON[values.Length];
        for (var i = 0; i < values.Length; i++) arguments[i] = BehaviorJSON.From(values[i]);
        try { return BehaviorJSON.Array(arguments).ToString(); }
        catch (BehaviorError) { throw GatewayFailure("invalid"); }
    }
    public static BehaviorJSON GatewayResult(string answer) {
        System.Text.Json.JsonElement envelope;
        try {
            using var document = System.Text.Json.JsonDocument.Parse(answer);
            envelope = document.RootElement.Clone();
        } catch { throw GatewayFailure("adapter"); }
        if (envelope.ValueKind != System.Text.Json.JsonValueKind.Object) throw GatewayFailure("adapter");
        if (envelope.TryGetProperty("result", out var result)) {
            try { return BehaviorJSON.Parse(result.GetRawText()); }
            catch (BehaviorError) { throw GatewayFailure("adapter"); }
        }
        if (!envelope.TryGetProperty("error", out var kind) || kind.ValueKind != System.Text.Json.JsonValueKind.String) throw GatewayFailure("adapter");
        if (System.Array.IndexOf(GatewayFailures, kind.GetString()) < 0) throw GatewayFailure("adapter");
        throw GatewayFailure(kind.GetString());
    }
    private static BehaviorError GatewayFailure(string kind) => new BehaviorError(kind, null, "Gateway call failed.");
    private static bool IsBlank(string value) {
        foreach (char character in value) if (!(character >= 9 && character <= 13) && character != 32 && character != 133 && character != 160 && character != 5760 && !(character >= 8192 && character <= 8202) && character != 8232 && character != 8233 && character != 8239 && character != 8287 && character != 12288) return false;
        return true;
    }
    private static BehaviorAdapters Configure(System.Type configuration, BehaviorExecution execution) { __TAC_BEHAVIOR_CONFIGURATIONS__ throw new BehaviorError("CONFIGURATION_UNKNOWN"); }
    public static BehaviorContext create(System.Type configuration, string partition, BehaviorSignal signal, int timeoutMs = 10000, string[] profiles = null, System.Collections.Generic.IDictionary<string, string> properties = null) {
        if (string.IsNullOrEmpty(partition) || System.Text.Encoding.UTF8.GetByteCount(partition) > 256 || signal == null || timeoutMs < 1 || timeoutMs > 60000) throw new BehaviorError("CONTEXT_INVALID");
        var execution = new BehaviorExecution(partition, signal, Now + timeoutMs); execution.Check();
        var selected = SnapshotProfiles(profiles); var values = SnapshotProperties(properties ?? new System.Collections.Generic.Dictionary<string, string>(), "configuration");
        ActiveConfiguration(TypePolicies(configuration), selected, values);
        BehaviorAdapters adapters;
        try { adapters = Configure(configuration, execution); } catch (BehaviorError) { throw; } catch { throw new BehaviorError("CONFIGURATION_FAILED"); }
        execution.Check(); if (adapters == null) throw new BehaviorError("CONFIGURATION_INVALID");
        return new BehaviorContext(adapters, partition, signal, timeoutMs);
    }
    private static bool Matches(string text, string pattern) => text != null && System.Text.RegularExpressions.Regex.IsMatch(text, pattern, System.Text.RegularExpressions.RegexOptions.CultureInvariant);
    private static string FrameworkPrefix => Environment == "tac" ? "TAC_" : "YON_";
    private static string[] SnapshotProfiles(string[] profiles) {
        if (profiles == null) return System.Array.Empty<string>();
        if (profiles.Length > 32) throw new BehaviorError("configuration");
        var selected = new System.Collections.Generic.HashSet<string>(System.StringComparer.Ordinal); var copy = new string[profiles.Length];
        for (var i = 0; i < profiles.Length; i++) { string value = profiles[i]; if (!Matches(value, @"\A[A-Za-z_$][A-Za-z0-9_$]{0,63}\z") || !selected.Add(value)) throw new BehaviorError("configuration"); copy[i] = value; }
        return copy;
    }
    private static System.Collections.Generic.Dictionary<string, string> SnapshotProperties(System.Collections.Generic.IDictionary<string, string> properties, string code) {
        try {
            if (properties == null || properties.Count > 256) throw new BehaviorError(code);
            var copy = new System.Collections.Generic.Dictionary<string, string>(System.StringComparer.Ordinal); var json = new System.Collections.Generic.Dictionary<string, BehaviorJSON>();
            foreach (var pair in properties) {
                if (pair.Key == null || pair.Key.Length > 256 || !pair.Key.StartsWith(FrameworkPrefix, System.StringComparison.Ordinal) || !Matches(pair.Key, @"\A[A-Z][A-Z0-9_]*\z") || pair.Value == null || System.Text.Encoding.UTF8.GetByteCount(pair.Value) > 4096) throw new BehaviorError(code);
                copy.Add(pair.Key, pair.Value); json.Add(pair.Key, BehaviorJSON.From(pair.Value));
            }
            BehaviorJSON.Object(json).Bound(65536); return copy;
        } catch { throw new BehaviorError(code); }
    }
    private static void ActiveConfiguration(BehaviorJSON policies, string[] profiles, System.Collections.Generic.Dictionary<string, string> values) {
        if (policies.Has("profiles")) {
            bool active = false; var configured = policies.Get("profiles").Items;
            if (configured.Length == 0 || configured.Length > 32) throw new BehaviorError("configuration");
            foreach (var profile in configured) { var name = profile.Text; if (!Matches(name, @"\A[A-Za-z_$][A-Za-z0-9_$]{0,63}\z")) throw new BehaviorError("configuration"); if (System.Array.IndexOf(profiles, name) >= 0) active = true; }
            if (!active) throw new BehaviorError("inactive-configuration");
        }
        if (policies.Has("conditionalOnProperty")) {
            var condition = policies.Get("conditionalOnProperty"); condition.Keys("name", "havingValue", "matchIfMissing"); string name = condition.Get("name").Text;
            if (name.Length > 256 || !name.StartsWith(FrameworkPrefix, System.StringComparison.Ordinal) || !Matches(name, @"\A[A-Z][A-Z0-9_]*\z") || System.Text.Encoding.UTF8.GetByteCount(condition.Get("havingValue").Text) > 4096) throw new BehaviorError("configuration");
            foreach (var key in values.Keys) if (key.StartsWith(name + "_", System.StringComparison.Ordinal)) throw new BehaviorError("inactive-configuration");
            bool active = values.TryGetValue(name, out var value) ? value == condition.Get("havingValue").Text : condition.Get("matchIfMissing").Boolean;
            if (!active) throw new BehaviorError("inactive-configuration");
        }
    }
    public static BehaviorJSON properties(System.Type type, System.Collections.Generic.IDictionary<string, string> values) {
        try {
            var supplied = SnapshotProperties(values, "configuration-properties"); var schema = TypePolicies(type).Get("configurationProperties"); schema.Keys("prefix", "fields");
            string prefix = schema.Get("prefix").Text;
            if (prefix.Length > 128 || !prefix.StartsWith(FrameworkPrefix, System.StringComparison.Ordinal) || !prefix.EndsWith("_", System.StringComparison.Ordinal) || !Matches(prefix, @"\A[A-Z][A-Z0-9_]*\z")) throw new BehaviorError("configuration-properties");
            var fields = schema.Get("fields").Members; var known = new System.Collections.Generic.HashSet<string>(System.StringComparer.Ordinal); var result = new System.Collections.Generic.Dictionary<string, BehaviorJSON>();
            foreach (var field in fields) {
                if (!Matches(field.Key, @"\A[A-Za-z_$][A-Za-z0-9_$]{0,63}\z")) throw new BehaviorError("configuration-properties");
                field.Value.Keys("name", "type", "default", "min", "max"); string suffix = field.Value.Get("name").Text;
                if (!Matches(suffix, @"\A[A-Z][A-Z0-9_]*\z") || prefix.Length + suffix.Length > 256 || !known.Add(prefix + suffix)) throw new BehaviorError("configuration-properties");
                bool present = supplied.TryGetValue(prefix + suffix, out var raw);
                BehaviorJSON value = present ? ParseProperty(raw, field.Value.Get("type").Text) : field.Value.Has("default") ? field.Value.Get("default") : throw new BehaviorError("configuration-properties");
                ValidateProperty(value, field.Value); if (field.Value.Get("type").Text == "integer") value = BehaviorJSON.From((long)value.Value.GetDouble()); result.Add(field.Key, value);
            }
            foreach (var key in supplied.Keys) if (key.StartsWith(prefix, System.StringComparison.Ordinal) && !known.Contains(key)) throw new BehaviorError("configuration-properties");
            var bound = BehaviorJSON.Object(result); bound.Bound(65536); return bound;
        } catch { throw new BehaviorError("configuration-properties"); }
    }
    private static System.Collections.Generic.Dictionary<string, string> ApplicationInputs(BehaviorJSON application) {
        var flat = new System.Collections.Generic.Dictionary<string, string>(System.StringComparer.Ordinal);
        application.Bound(65536);
        if (!application.Members.ContainsKey("tac") && !application.Members.ContainsKey("yon")) throw new BehaviorError("configuration-properties");
        foreach (var item in application.Members) {
            if (item.Key != "tac" && item.Key != "yon") continue;
            _ = item.Value.Members; FlattenApplication(item.Value, item.Key, 0, flat);
        }
        return flat;
    }
    private static void FlattenApplication(BehaviorJSON value, string path, int depth, System.Collections.Generic.Dictionary<string, string> flat) {
        if (depth > 16 || path.Length > 256 || flat.Count >= 256) throw new BehaviorError("configuration-properties");
        if (path == "yon.server.port") _ = value.Integer;
        if (value.Value.ValueKind == System.Text.Json.JsonValueKind.Object) {
            if (depth > 0 && value.Members.Count == 0) flat.Add(path + ".", "");
            foreach (var item in value.Members) {
                if (!Matches(item.Key, @"\A[a-z][a-z0-9]*\z")) throw new BehaviorError("configuration-properties");
                FlattenApplication(item.Value, path + "." + item.Key, depth + 1, flat);
            }
        } else {
            if (value.Value.ValueKind == System.Text.Json.JsonValueKind.Null || value.Value.ValueKind == System.Text.Json.JsonValueKind.Array) { flat.Add(path + ".", ""); return; }
            string text;
            switch (value.Value.ValueKind) {
                case System.Text.Json.JsonValueKind.String: text = value.Text; break;
                case System.Text.Json.JsonValueKind.True: text = "true"; break;
                case System.Text.Json.JsonValueKind.False: text = "false"; break;
                case System.Text.Json.JsonValueKind.Number: text = value.Value.GetDouble().ToString("G17", System.Globalization.CultureInfo.InvariantCulture); break;
                default: throw new BehaviorError("configuration-properties");
            }
            if (System.Text.Encoding.UTF8.GetByteCount(text) > 4096 || flat.Count >= 256) throw new BehaviorError("configuration-properties");
            flat.Add(path, text);
        }
    }
    private static System.Collections.Generic.Dictionary<string, string> ApplicationProperties(BehaviorJSON application) {
        var result = new System.Collections.Generic.Dictionary<string, string>(System.StringComparer.Ordinal);
        foreach (var item in ApplicationInputs(application)) if (item.Key.StartsWith(Environment + ".", System.StringComparison.Ordinal))
            result.Add(item.Key.Replace('.', '_').ToUpperInvariant(), item.Value);
        return result;
    }
    public static BehaviorJSON properties(System.Type type, BehaviorJSON application) => properties(type, ApplicationProperties(application));
    public static BehaviorContext create(System.Type configuration, string partition, BehaviorSignal signal, BehaviorJSON properties, int timeoutMs = 10000, string[] profiles = null) =>
        create(configuration, partition, signal, timeoutMs, profiles, ApplicationProperties(properties));
    public static BehaviorJSON values(System.Type type, BehaviorJSON application) {
        try {
            var flat = ApplicationInputs(application); var schema = TypePolicies(type).Get("values"); var result = new System.Collections.Generic.Dictionary<string, BehaviorJSON>();
            foreach (var field in schema.Members) {
                if (!Matches(field.Key, @"\A[A-Za-z_$][A-Za-z0-9_$]{0,127}\z")) throw new BehaviorError("configuration-properties");
                field.Value.Keys("path", "type", "default", "min", "max"); string path = field.Value.Get("path").Text;
                foreach (var key in flat.Keys) if (key.StartsWith(path + ".", System.StringComparison.Ordinal)) throw new BehaviorError("configuration-properties");
                bool present = flat.TryGetValue(path, out var raw);
                BehaviorJSON value = present ? ParseProperty(raw, field.Value.Get("type").Text) : field.Value.Has("default") ? field.Value.Get("default") : throw new BehaviorError("configuration-properties");
                ValidateProperty(value, field.Value); if (field.Value.Get("type").Text == "integer") value = BehaviorJSON.From((long)value.Value.GetDouble()); result.Add(field.Key, value);
            }
            var bound = BehaviorJSON.Object(result); bound.Bound(65536); return bound;
        } catch { throw new BehaviorError("configuration-properties"); }
    }
    private static BehaviorJSON ParseProperty(string raw, string type) {
        if (type == "string") return BehaviorJSON.From(raw);
        if (type == "boolean") { if (raw != "true" && raw != "false") throw new BehaviorError("configuration-properties"); return BehaviorJSON.From(raw == "true"); }
        string grammar = type == "integer" ? @"\A-?(0|[1-9][0-9]*)\z" : @"\A-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?\z";
        if ((type != "integer" && type != "number") || !Matches(raw, grammar)) throw new BehaviorError("configuration-properties");
        return BehaviorJSON.Parse(raw);
    }
    private static void ValidateProperty(BehaviorJSON value, BehaviorJSON descriptor) {
        string type = descriptor.Get("type").Text; bool numeric = type == "integer" || type == "number";
        if (!numeric && (descriptor.Has("min") || descriptor.Has("max"))) throw new BehaviorError("configuration-properties");
        if (type == "string") { if (System.Text.Encoding.UTF8.GetByteCount(value.Text) > 4096) throw new BehaviorError("configuration-properties"); return; }
        if (type == "boolean") { _ = value.Boolean; return; }
        if (!numeric || value.Value.ValueKind != System.Text.Json.JsonValueKind.Number) throw new BehaviorError("configuration-properties");
        double number = value.Value.GetDouble(); if (!double.IsFinite(number) || System.Math.Abs(number) > 9007199254740991d || (type == "integer" && System.Math.Truncate(number) != number)) throw new BehaviorError("configuration-properties");
        double minimum = descriptor.Has("min") ? descriptor.Get("min").Value.GetDouble() : -9007199254740991d;
        double maximum = descriptor.Has("max") ? descriptor.Get("max").Value.GetDouble() : 9007199254740991d;
        if (!double.IsFinite(minimum) || !double.IsFinite(maximum) || System.Math.Abs(minimum) > 9007199254740991d || System.Math.Abs(maximum) > 9007199254740991d || minimum > maximum || number < minimum || number > maximum) throw new BehaviorError("configuration-properties");
    }
}
internal static class BehaviorOpenTransactions {
    private static readonly System.Threading.AsyncLocal<System.Collections.Generic.Dictionary<string, string>> open = new System.Threading.AsyncLocal<System.Collections.Generic.Dictionary<string, string>>();
    public static string Of(string adapter) => open.Value != null && open.Value.TryGetValue(adapter, out var identifier) ? identifier : null;
    public static async System.Threading.Tasks.Task<BehaviorJSON> Within(string adapter, string identifier, System.Func<System.Threading.Tasks.Task<BehaviorJSON>> body) {
        var outer = open.Value;
        var joined = outer == null ? new System.Collections.Generic.Dictionary<string, string>() : new System.Collections.Generic.Dictionary<string, string>(outer);
        joined[adapter] = identifier;
        open.Value = joined;
        try { return await body(); } finally { open.Value = outer; }
    }
}
public sealed class BehaviorContext : System.IDisposable {
    private readonly BehaviorAdapters adapters;
    private readonly string partition;
    private readonly BehaviorSignal owner;
    private readonly BehaviorSignal signal = new BehaviorSignal();
    private readonly int timeoutMs;
    private readonly System.Threading.CancellationTokenRegistration cancellation;
    private int active;
    internal BehaviorContext(BehaviorAdapters adapters, string partition, BehaviorSignal owner, int timeoutMs) {
        this.adapters = new BehaviorAdapters();
        foreach (var pair in adapters.caches) this.adapters.caches.Add(pair.Key, pair.Value);
        foreach (var pair in adapters.http) this.adapters.http.Add(pair.Key, pair.Value);
        foreach (var pair in adapters.entities) this.adapters.entities.Add(pair.Key, pair.Value);
        this.adapters.authorize = adapters.authorize;
        this.adapters.authorizeEntity = adapters.authorizeEntity;
        this.partition = partition; this.owner = owner; this.timeoutMs = timeoutMs;
        cancellation = owner.Token.Register(() => signal.Abort());
    }
    public void dispose() { signal.Abort(); cancellation.Dispose(); }
    public void Dispose() => dispose();
    public T Paginate<T>(System.Func<T> body) {
        if (System.Threading.Interlocked.Increment(ref active) > 64) { System.Threading.Interlocked.Decrement(ref active); throw new BehaviorError("OPERATION_LIMIT"); }
        try { var execution = new BehaviorExecution(partition, signal, Behaviors.Now + timeoutMs); execution.Check(); if (owner.Aborted) throw new BehaviorError("CANCELLED"); T result = body(); execution.Check(); return result; }
        finally { System.Threading.Interlocked.Decrement(ref active); }
    }
    private async System.Threading.Tasks.Task<BehaviorJSON> Operation(System.Func<BehaviorExecution, System.Threading.Tasks.Task<BehaviorJSON>> action) {
        if (System.Threading.Interlocked.Increment(ref active) > 64) { System.Threading.Interlocked.Decrement(ref active); throw new BehaviorError("OPERATION_LIMIT"); }
        try { var execution = new BehaviorExecution(partition, signal, Behaviors.Now + timeoutMs); execution.Check(); if (owner.Aborted) throw new BehaviorError("CANCELLED"); var result = await action(execution); execution.Check(); result.Bound(1048576); return result; }
        finally { System.Threading.Interlocked.Decrement(ref active); }
    }
    private async System.Threading.Tasks.Task<T> Adapter<T>(BehaviorExecution execution, System.Func<System.Threading.Tasks.Task<T>> action) {
        execution.Check(); T result; try { result = await action(); } catch (BehaviorError) { throw; } catch { execution.Check(); throw new BehaviorError("ADAPTER_FAILED"); } execution.Check(); if (result == null) throw new BehaviorError("ADAPTER_INVALID"); return result;
    }
    private BehaviorCacheAdapter CacheAdapter(string name) => adapters.caches.TryGetValue(name, out var cache) ? cache : BehaviorStorage.Cache(name) ?? throw new BehaviorError("CACHE_MISSING");
    private BehaviorEntityAdapter EntityAdapter(string name) => adapters.entities.TryGetValue(name, out var adapter) ? adapter : BehaviorStorage.Entities(name) ?? throw new BehaviorError("ENTITY_MISSING");
    private static BehaviorJSON Method(string type, string method) { var metadata = Behaviors.Registration(type).Get("methods").Get(method); if (metadata.IsNull) throw new BehaviorError("METHOD_UNKNOWN"); return metadata; }
    private static BehaviorJSON[] Selected(BehaviorJSON metadata, BehaviorJSON[] arguments) {
        var values = new System.Collections.Generic.List<BehaviorJSON>();
        foreach (var item in metadata.Get("cacheable").Get("keyArgs").Items) { var index = item.Integer; if (index < 0 || index >= arguments.Length) throw new BehaviorError("CACHE_KEY"); values.Add(arguments[index]); }
        return values.ToArray();
    }
    private async System.Threading.Tasks.Task<string> Authorize(string operation, string type, string method, BehaviorJSON policy, BehaviorJSON metadata, BehaviorJSON[] arguments, BehaviorExecution execution) {
        var selected = BehaviorJSON.Array(Selected(metadata, arguments));
        var key = BehaviorJSON.Array(BehaviorJSON.From(partition), policy.Get("name"), BehaviorJSON.From(Behaviors.Module), BehaviorJSON.From(type), BehaviorJSON.From(method), selected); key.Bound(4096);
        var call = BehaviorJSON.Object(new System.Collections.Generic.Dictionary<string, BehaviorJSON> { ["operation"] = BehaviorJSON.From(operation), ["cache"] = policy.Get("name"), ["type"] = BehaviorJSON.From(type), ["method"] = BehaviorJSON.From(method), ["keyArguments"] = selected });
        if (adapters.authorize == null && !adapters.caches.ContainsKey(policy.Get("name").Text) && BehaviorStorage.Cache(policy.Get("name").Text) != null) return key.ToString();
        if (adapters.authorize == null || !await Adapter(execution, () => adapters.authorize(call, execution))) throw new BehaviorError("FORBIDDEN");
        return key.ToString();
    }
    public System.Threading.Tasks.Task<BehaviorJSON> Invoke(string type, string method, BehaviorJSON[] arguments, System.Func<System.Threading.Tasks.Task<BehaviorJSON>> body) => Operation(async execution => {
        var metadata = Method(type, method);
        if (metadata.Get("httpExchange").Has("generated")) {
            var declared = ExchangeRequest(metadata.Get("httpExchange"), arguments);
            body = () => System.Threading.Tasks.Task.FromResult(declared);
            metadata = WithFilledPath(metadata, arguments);
        }
        if (metadata.Has("cacheEvict") || metadata.Has("cachePut")) return await Mutation(type, metadata, arguments, body, execution);
        if (!metadata.Has("cacheable")) return await Resilient(type, method, metadata, body, execution);
        var policy = Behaviors.Registration(type).Get("cache");
        string key = await Authorize("cache:read", type, method, policy, metadata, arguments, execution);
        var cache = CacheAdapter(policy.Get("name").Text);
        var entry = await Adapter(execution, () => cache.get(key, execution)); entry.Bound(1048576); entry.Keys("hit", "value", "expiresAt");
        if (entry.Get("hit").Boolean) { var expiry = entry.Get("expiresAt").Integer; if (expiry > Behaviors.Now + policy.Get("ttlMs").Integer) throw new BehaviorError("CACHE_EXPIRY"); if (expiry > Behaviors.Now) { var hit = entry.Get("value"); if (!entry.Has("value")) throw new BehaviorError("CACHE_VALUE"); hit.Bound((int)policy.Get("maxBytes").Integer); return hit; } }
        var result = await Resilient(type, method, metadata, body, execution); execution.Check(); result.Bound((int)policy.Get("maxBytes").Integer);
        await Adapter(execution, async () => { await cache.set(key, result, policy, Behaviors.Now + policy.Get("ttlMs").Integer, execution); return true; }); return result;
    });
    private static BehaviorJSON WithFilledPath(BehaviorJSON metadata, BehaviorJSON[] arguments) {
        var policy = metadata.Get("httpExchange");
        if (!policy.Has("pathVariables")) return metadata;
        string path = policy.Get("path").Text;
        foreach (var pair in policy.Get("pathVariables").Members) {
            long index = pair.Value.Integer;
            if (index < 0 || index >= arguments.Length) throw new BehaviorError("HTTP_ARGUMENT");
            var value = arguments[index];
            string text;
            try { text = value.Text; } catch (BehaviorError) { text = value.ToString(); }
            if (!System.Text.RegularExpressions.Regex.IsMatch(text, "^[A-Za-z0-9._~-]{1,256}$") || text == "." || text == "..") throw new BehaviorError("HTTP_ARGUMENT");
            path = path.Replace("{" + pair.Key + "}", text);
        }
        var exchange = new System.Collections.Generic.Dictionary<string, BehaviorJSON>(policy.Members) { ["path"] = BehaviorJSON.From(path) };
        var copy = new System.Collections.Generic.Dictionary<string, BehaviorJSON>(metadata.Members) { ["httpExchange"] = BehaviorJSON.Object(exchange) };
        return BehaviorJSON.Object(copy);
    }
    private static BehaviorJSON ExchangeRequest(BehaviorJSON policy, BehaviorJSON[] arguments) {
        BehaviorJSON Pick(long index) {
            if (index < 0 || index >= arguments.Length) throw new BehaviorError("HTTP_ARGUMENT");
            return arguments[index];
        }
        var query = new System.Collections.Generic.Dictionary<string, BehaviorJSON>();
        var headers = new System.Collections.Generic.Dictionary<string, BehaviorJSON>();
        if (policy.Has("query")) foreach (var pair in policy.Get("query").Members) query[pair.Key] = Pick(pair.Value.Integer);
        if (policy.Has("headers")) foreach (var pair in policy.Get("headers").Members) headers[pair.Key] = Pick(pair.Value.Integer);
        var descriptor = new System.Collections.Generic.Dictionary<string, BehaviorJSON> {
            ["query"] = BehaviorJSON.Object(query), ["headers"] = BehaviorJSON.Object(headers),
        };
        if (policy.Has("body")) descriptor["body"] = Pick(policy.Get("body").Integer);
        return BehaviorJSON.Object(descriptor);
    }
    private static BehaviorJSON[] MutationArguments(BehaviorJSON mutation, BehaviorJSON[] arguments) {
        var mapping = mutation.Get("arguments").Items;
        var normalized = new BehaviorJSON[mapping.Length];
        for (var i = 0; i < mapping.Length; i++) { long index = mapping[i].Integer; if (index < 0 || index >= arguments.Length) throw new BehaviorError("CACHE_ARGUMENT"); normalized[i] = arguments[index]; }
        var target = mutation.Get("target");
        if (target.Has("pageable")) { var page = target.Get("pageable"); long index = page.Get("argument").Integer; if (index < 0 || index >= normalized.Length) throw new BehaviorError("PAGE_ARGUMENT"); normalized[index] = BehaviorPage.Normalize(normalized[index], page); }
        return normalized;
    }
    private async System.Threading.Tasks.Task<BehaviorJSON> Mutation(string type, BehaviorJSON metadata, BehaviorJSON[] arguments, System.Func<System.Threading.Tasks.Task<BehaviorJSON>> body, BehaviorExecution execution) {
        bool put = metadata.Has("cachePut"); var mutation = metadata.Get(put ? "cachePut" : "cacheEvict"); var target = mutation.Get("target");
        var policy = Behaviors.Registration(type).Get("cache"); var normalized = MutationArguments(mutation, arguments);
        string key = await Authorize(put ? "cache:put" : "cache:evict", type, target.Get("method").Text, policy, target, normalized, execution);
        var cache = CacheAdapter(policy.Get("name").Text);
        var result = await Resilient(type, target.Get("method").Text, metadata, body, execution); execution.Check(); result.Bound(put ? (int)policy.Get("maxBytes").Integer : 1048576);
        if (put) await Adapter(execution, async () => { await cache.set(key, result, policy, Behaviors.Now + policy.Get("ttlMs").Integer, execution); return true; });
        else await Adapter(execution, async () => { await cache.delete(key, execution); return true; });
        return result;
    }
    private readonly System.Collections.Concurrent.ConcurrentDictionary<string, System.Threading.SemaphoreSlim> slots = new System.Collections.Concurrent.ConcurrentDictionary<string, System.Threading.SemaphoreSlim>();
    private async System.Threading.Tasks.Task<BehaviorJSON> Transacted(BehaviorJSON metadata, System.Func<System.Threading.Tasks.Task<BehaviorJSON>> body, BehaviorExecution execution) {
        if (!metadata.Has("transactional")) return await Attempt(metadata, body, execution);
        var name = metadata.Get("transactional").Get("adapter").Text;
        if (BehaviorOpenTransactions.Of(name) != null) return await Attempt(metadata, body, execution);
        var adapter = EntityAdapter(name);
        var begun = await Control(adapter, execution, "begin", null);
        var identifier = begun.Has("transaction") ? begun.Get("transaction").Text : "";
        if (identifier.Length == 0 || System.Text.Encoding.UTF8.GetByteCount(identifier) > 128) throw new BehaviorError("ENTITY_TRANSACTION");
        BehaviorJSON value;
        try { value = await BehaviorOpenTransactions.Within(name, identifier, () => Attempt(metadata, body, execution)); }
        catch {
            try { await Control(adapter, execution, "rollback", identifier); } catch { }
            throw;
        }
        await Control(adapter, execution, "commit", identifier);
        return value;
    }
    private System.Threading.Tasks.Task<BehaviorJSON> Control(BehaviorEntityAdapter adapter, BehaviorExecution execution, string action, string identifier) {
        var command = new System.Collections.Generic.Dictionary<string, BehaviorJSON> { ["operation"] = BehaviorJSON.From("transaction"), ["action"] = BehaviorJSON.From(action) };
        if (identifier != null) command["transaction"] = BehaviorJSON.From(identifier);
        return Adapter(execution, () => adapter.execute(BehaviorJSON.Object(command), execution));
    }
    private static bool Retryable(System.Exception error) => !(error is BehaviorError failure) || failure.Code == "ADAPTER_FAILED";
    private async System.Threading.Tasks.Task<BehaviorJSON> Resilient(string type, string method, BehaviorJSON metadata, System.Func<System.Threading.Tasks.Task<BehaviorJSON>> body, BehaviorExecution execution) {
        if (!metadata.Has("concurrencyLimit")) return await Transacted(metadata, body, execution);
        var limit = metadata.Get("concurrencyLimit");
        var slot = slots.GetOrAdd(type + "." + method, _ => new System.Threading.SemaphoreSlim((int)limit.Get("limit").Integer));
        var waiting = limit.Get("policy").Text == "block" ? (int)System.Math.Max(0, execution.Deadline - Behaviors.Now) : 0;
        if (!await slot.WaitAsync(waiting)) throw new BehaviorError(waiting > 0 ? "DEADLINE" : "OPERATION_LIMIT");
        try { return await Transacted(metadata, body, execution); } finally { slot.Release(); }
    }
    private async System.Threading.Tasks.Task<BehaviorJSON> Attempt(BehaviorJSON metadata, System.Func<System.Threading.Tasks.Task<BehaviorJSON>> body, BehaviorExecution execution) {
        if (!metadata.Has("retryable")) return await Body(metadata, body, execution);
        var policy = metadata.Get("retryable");
        var maxRetries = policy.Get("maxRetries").Integer;
        for (var attempt = 0; ; attempt++) {
            try { return await Body(metadata, body, execution); }
            catch (System.Exception error) when (attempt < maxRetries && Retryable(error)) {
                execution.Check();
                var delay = System.Math.Min(policy.Get("delayMs").Integer * System.Math.Pow(policy.Get("multiplier").Number, attempt), policy.Get("maxDelayMs").Integer);
                var wait = (int)System.Math.Round(delay + new System.Random().NextDouble() * policy.Get("jitterMs").Integer);
                if (wait > 0) await System.Threading.Tasks.Task.Delay(wait);
                execution.Check();
            }
        }
    }
    private async System.Threading.Tasks.Task<BehaviorJSON> Body(BehaviorJSON metadata, System.Func<System.Threading.Tasks.Task<BehaviorJSON>> body, BehaviorExecution execution) {
        execution.Check(); var result = await body(); execution.Check(); result.Bound(1048576);
        if (!metadata.Has("httpExchange")) return result;
        result.Keys("query", "headers", "body"); var policy = metadata.Get("httpExchange");
        if ((policy.Get("method").Text == "GET" || policy.Get("method").Text == "HEAD") && result.Has("body")) throw new BehaviorError("HTTP_BODY");
        if (!adapters.http.TryGetValue(policy.Get("client").Text, out var client)) throw new BehaviorError("HTTP_MISSING");
        var request = result.Members; if (!request.ContainsKey("query")) request["query"] = BehaviorJSON.Parse("{}"); if (!request.ContainsKey("headers")) request["headers"] = BehaviorJSON.Parse("{}"); request["method"] = policy.Get("method"); request["path"] = policy.Get("path"); request["redirect"] = BehaviorJSON.From("error"); request["credentials"] = BehaviorJSON.From("omit"); request["deadline"] = BehaviorJSON.From(execution.Deadline); request["maxResponseBytes"] = BehaviorJSON.From(1048576);
        ValidateHTTP(result); return await Adapter(execution, () => client.exchange(BehaviorJSON.Object(request), execution));
    }
    private static void ValidateHTTP(BehaviorJSON descriptor) {
        if (descriptor.Has("headers")) foreach (var pair in descriptor.Get("headers").Members) {
            if (pair.Key.Length == 0 || pair.Key.Length > 128 || !System.Text.RegularExpressions.Regex.IsMatch(pair.Key, "^[!#$%&'*+.^_`|~0-9A-Za-z-]+$")) throw new BehaviorError("HTTP_HEADER");
            var value = pair.Value.Text; if (value.Length > 8192 || value.Contains('\r') || value.Contains('\n') || value.Contains('\0')) throw new BehaviorError("HTTP_HEADER");
            if (System.Array.IndexOf(new[] { "host", "connection", "content-length", "transfer-encoding", "cookie" }, pair.Key.ToLowerInvariant()) >= 0) throw new BehaviorError("HTTP_HEADER");
        }
        if (descriptor.Has("query")) foreach (var pair in descriptor.Get("query").Members) { if (pair.Key.Length > 256 || pair.Value.ToString().Length > 8192 || pair.Value.IsNull || pair.Value.Value.ValueKind == System.Text.Json.JsonValueKind.Object || pair.Value.Value.ValueKind == System.Text.Json.JsonValueKind.Array) throw new BehaviorError("HTTP_QUERY"); }
    }
    public System.Threading.Tasks.Task<BehaviorJSON> evict(string type, string method, params BehaviorJSON[] arguments) => Operation(async execution => {
        var metadata = Method(type, method); if (!metadata.Has("cacheable")) throw new BehaviorError("CACHE_METHOD");
        if (metadata.Has("pageable")) { var page = metadata.Get("pageable"); int index = (int)page.Get("argument").Integer; if (index >= arguments.Length) throw new BehaviorError("PAGE_ARGUMENT"); arguments = (BehaviorJSON[])arguments.Clone(); arguments[index] = BehaviorPage.Normalize(arguments[index], page); }
        var policy = Behaviors.Registration(type).Get("cache"); string key = await Authorize("cache:evict", type, method, policy, metadata, arguments, execution);
        var cache = CacheAdapter(policy.Get("name").Text);
        await Adapter(execution, async () => { await cache.delete(key, execution); return true; }); return BehaviorJSON.From(null);
    });
    public BehaviorRestClient restClient(string name) {
        if (string.IsNullOrEmpty(name) || name.Length > 128) throw new BehaviorError("HTTP_MISSING");
        return new BehaviorRestClient(this, name);
    }
    internal System.Threading.Tasks.Task<BehaviorJSON> RestExchange(string name, string method, string path, BehaviorJSON descriptor) => Operation(async execution => {
        if (!adapters.http.TryGetValue(name, out var client)) throw new BehaviorError("HTTP_MISSING");
        descriptor.Keys("query", "headers", "body");
        if ((method == "GET" || method == "HEAD") && descriptor.Has("body")) throw new BehaviorError("HTTP_BODY");
        ValidateHTTP(descriptor);
        var request = descriptor.Members;
        if (!request.ContainsKey("query")) request["query"] = BehaviorJSON.Parse("{}");
        if (!request.ContainsKey("headers")) request["headers"] = BehaviorJSON.Parse("{}");
        request["method"] = BehaviorJSON.From(method); request["path"] = BehaviorJSON.From(path);
        request["redirect"] = BehaviorJSON.From("error"); request["credentials"] = BehaviorJSON.From("omit");
        request["deadline"] = BehaviorJSON.From(execution.Deadline); request["maxResponseBytes"] = BehaviorJSON.From(1048576);
        return await Adapter(execution, () => client.exchange(BehaviorJSON.Object(request), execution));
    });
    internal static bool SafePath(string path) {
        if (string.IsNullOrEmpty(path) || !path.StartsWith("/") || path.StartsWith("//") || path.Length > 2048) return false;
        foreach (var character in path) if (character <= ' ' || character == '\\' || character == '?' || character == '#' || character == '%') return false;
        foreach (var part in path.Split('/')) if (part == "." || part == "..") return false;
        return true;
    }
    public BehaviorJSON MapValue(BehaviorJSON plan, BehaviorJSON value) {
        if (value.Value.ValueKind != System.Text.Json.JsonValueKind.Object) throw new BehaviorError("JSON_OBJECT");
        var output = new System.Collections.Generic.Dictionary<string, BehaviorJSON>();
        foreach (var field in plan.Get("fields").Items) {
            if (!field.Has("source")) { output[field.Get("target").Text] = field.Get("constant"); continue; }
            var current = value;
            foreach (var name in field.Get("source").Text.Split('.')) current = current.Get(name);
            output[field.Get("target").Text] = current;
        }
        var result = BehaviorJSON.Object(output); result.Bound(1048576); return result;
    }
    public BehaviorRepository repository(System.Type entity) => new BehaviorRepository(this, Behaviors.Entity(entity));
    public System.Threading.Tasks.Task<BehaviorJSON> Query(BehaviorJSON metadata, BehaviorJSON[] arguments) => Operation(async execution => {
        var command = QueryCommand(metadata, arguments);
        var entityAdapter = metadata.Get("entity").Get("adapter").Text;
        var transaction = BehaviorOpenTransactions.Of(entityAdapter);
        if (transaction != null) { var joined = command.Members; joined["transaction"] = BehaviorJSON.From(transaction); command = BehaviorJSON.Object(joined); }
        var builtin = adapters.authorizeEntity == null && !adapters.entities.ContainsKey(entityAdapter) && BehaviorStorage.Entities(entityAdapter) != null;
        if (!builtin && (adapters.authorizeEntity == null || !await Adapter(execution, () => adapters.authorizeEntity(command, execution)))) throw new BehaviorError("FORBIDDEN");
        var adapter = EntityAdapter(entityAdapter);
        var result = await Adapter(execution, () => adapter.execute(command, execution)); result.Bound(1048576);
        string kind = metadata.Get("operation").Text;
        if (kind == "find" || kind == "findOne") { var rows = result.Items; if (rows.Length > command.Get("query").Get("page").Get("size").Integer) throw new BehaviorError("QUERY_RESULT"); foreach (var row in rows) _ = row.Members; if (kind == "findOne") return rows.Length == 0 ? BehaviorJSON.Parse("null") : rows[0]; }
        else if (kind == "count" || kind == "delete") { if (result.Integer < 0) throw new BehaviorError("QUERY_RESULT"); }
        else if (kind == "exists") { _ = result.Boolean; }
        else throw new BehaviorError("QUERY_KIND");
        return result;
    });
    private static BehaviorJSON QueryScalar(BehaviorJSON value) {
        if (value.Value.ValueKind == System.Text.Json.JsonValueKind.Object || value.Value.ValueKind == System.Text.Json.JsonValueKind.Array) throw new BehaviorError("QUERY_BINDING");
        value.Bound(65536); return value;
    }
    private static BehaviorJSON QueryBinding(BehaviorJSON condition, BehaviorJSON[] arguments) {
        long index = condition.Get("argument").Integer;
        if (index < 0 || index >= arguments.Length) throw new BehaviorError("QUERY_ARGUMENT");
        var value = arguments[index];
        if (condition.Get("operator").Text != "in") return QueryScalar(value);
        var items = value.Items; if (items.Length > 100) throw new BehaviorError("QUERY_BINDING");
        foreach (var item in items) QueryScalar(item); value.Bound(65536); return value;
    }
    private static BehaviorJSON QueryCommand(BehaviorJSON metadata, BehaviorJSON[] arguments) {
        var fields = new System.Collections.Generic.HashSet<string>(System.StringComparer.Ordinal);
        foreach (var field in metadata.Get("fields").Items) fields.Add(field.Text);
        var where = new System.Collections.Generic.List<BehaviorJSON>();
        foreach (var condition in metadata.Get("where").Items) {
            string field = condition.Get("field").Text; string op = condition.Get("operator").Text;
            if (!fields.Contains(field) || System.Array.IndexOf(new[] { "eq", "ne", "lt", "lte", "gt", "gte", "in", "isNull", "isNotNull" }, op) < 0) throw new BehaviorError("QUERY_BINDING");
            var bound = new System.Collections.Generic.Dictionary<string, BehaviorJSON> { ["field"] = BehaviorJSON.From(field), ["operator"] = BehaviorJSON.From(op) };
            if (op != "isNull" && op != "isNotNull") bound["value"] = QueryBinding(condition, arguments);
            where.Add(BehaviorJSON.Object(bound));
        }
        var bindings = BehaviorJSON.Array(where.ToArray()); bindings.Bound(65536);
        var order = metadata.Get("orderBy");
        foreach (var item in order.Items) { if (!fields.Contains(item.Get("field").Text) || System.Array.IndexOf(new[] { "asc", "desc" }, item.Get("direction").Text) < 0) throw new BehaviorError("QUERY_ORDER"); }
        string kind = metadata.Get("operation").Text;
        if (System.Array.IndexOf(new[] { "find", "findOne", "count", "exists", "delete" }, kind) < 0) throw new BehaviorError("QUERY_KIND");
        var query = new System.Collections.Generic.Dictionary<string, BehaviorJSON> { ["kind"] = BehaviorJSON.From(kind == "findOne" ? "find" : kind), ["where"] = bindings, ["orderBy"] = order };
        if (kind == "findOne") query["page"] = BehaviorPage.Normalize(BehaviorJSON.Parse("{\"size\":1}"), BehaviorJSON.Object(new System.Collections.Generic.Dictionary<string, BehaviorJSON> { ["defaultSize"] = BehaviorJSON.From(1), ["maxSize"] = BehaviorJSON.From(1), ["sort"] = metadata.Get("fields") }));
        if (kind == "find") {
            var input = BehaviorJSON.Parse("{}");
            if (!metadata.Get("pageArgument").IsNull) { long index = metadata.Get("pageArgument").Integer; if (index < 0 || index >= arguments.Length) throw new BehaviorError("QUERY_ARGUMENT"); input = arguments[index]; }
            var page = BehaviorPage.Normalize(input, BehaviorJSON.Object(new System.Collections.Generic.Dictionary<string, BehaviorJSON> { ["defaultSize"] = metadata.Get("defaultSize"), ["maxSize"] = metadata.Get("maxSize"), ["sort"] = metadata.Get("fields") }));
            if (page.Get("sort").Items.Length > 0) throw new BehaviorError("QUERY_ORDER"); query["page"] = page;
        }
        var entity = metadata.Get("entity");
        return BehaviorJSON.Object(new System.Collections.Generic.Dictionary<string, BehaviorJSON> { ["operation"] = BehaviorJSON.From("query"), ["entity"] = entity.Get("name"), ["table"] = entity.Get("table"), ["idField"] = entity.Get("id"), ["query"] = BehaviorJSON.Object(query) });
    }
    internal System.Threading.Tasks.Task<BehaviorJSON> ExecuteEntity(BehaviorJSON mapping, string operation, BehaviorJSON value) => Operation(async execution => {
        var adapter = EntityAdapter(mapping.Get("adapter").Text);
        var transaction = BehaviorOpenTransactions.Of(mapping.Get("adapter").Text);
        var command = mapping.Members; command.Remove("adapter"); command["operation"] = BehaviorJSON.From(operation); command[operation == "find" ? "page" : operation == "save" ? "value" : "id"] = value;
        if (transaction != null) command["transaction"] = BehaviorJSON.From(transaction);
        return await Adapter(execution, () => adapter.execute(BehaviorJSON.Object(command), execution));
    });
}
public sealed class BehaviorRestClient {
    private readonly BehaviorContext context; private readonly string name;
    internal BehaviorRestClient(BehaviorContext context, string name) { this.context = context; this.name = name; }
    public BehaviorRestCall get(string path) => new BehaviorRestCall(context, name, "GET", path);
    public BehaviorRestCall post(string path) => new BehaviorRestCall(context, name, "POST", path);
    public BehaviorRestCall put(string path) => new BehaviorRestCall(context, name, "PUT", path);
    public BehaviorRestCall patch(string path) => new BehaviorRestCall(context, name, "PATCH", path);
    public BehaviorRestCall delete(string path) => new BehaviorRestCall(context, name, "DELETE", path);
    public BehaviorRestCall head(string path) => new BehaviorRestCall(context, name, "HEAD", path);
    public BehaviorRestCall options(string path) => new BehaviorRestCall(context, name, "OPTIONS", path);
}
public sealed class BehaviorRestCall {
    private readonly BehaviorContext context; private readonly string name, method, path;
    private readonly System.Collections.Generic.Dictionary<string, BehaviorJSON> parameters = new System.Collections.Generic.Dictionary<string, BehaviorJSON>();
    private readonly System.Collections.Generic.Dictionary<string, BehaviorJSON> fields = new System.Collections.Generic.Dictionary<string, BehaviorJSON>();
    private BehaviorJSON payload; private bool hasBody;
    internal BehaviorRestCall(BehaviorContext context, string name, string method, string path) {
        if (!BehaviorContext.SafePath(path)) throw new BehaviorError("HTTP_PATH");
        this.context = context; this.name = name; this.method = method; this.path = path;
    }
    public BehaviorRestCall query(BehaviorJSON values) {
        foreach (var pair in values.Members) parameters[pair.Key] = pair.Value;
        return this;
    }
    public BehaviorRestCall header(string key, string value) { fields[key] = BehaviorJSON.From(value); return this; }
    public BehaviorRestCall body(BehaviorJSON value) {
        if (method == "GET" || method == "HEAD") throw new BehaviorError("HTTP_BODY");
        payload = value; hasBody = true; return this;
    }
    public System.Threading.Tasks.Task<BehaviorJSON> retrieve() {
        var descriptor = new System.Collections.Generic.Dictionary<string, BehaviorJSON> {
            ["query"] = BehaviorJSON.Object(parameters), ["headers"] = BehaviorJSON.Object(fields),
        };
        if (hasBody) descriptor["body"] = payload;
        return context.RestExchange(name, method, path, BehaviorJSON.Object(descriptor));
    }
}
public sealed class BehaviorRepository {
    private readonly BehaviorContext context; private readonly BehaviorJSON mapping;
    internal BehaviorRepository(BehaviorContext context, BehaviorJSON mapping) { this.context = context; this.mapping = mapping; }
    public System.Threading.Tasks.Task<BehaviorJSON> find(BehaviorJSON page) => context.ExecuteEntity(mapping, "find", BehaviorPage.Normalize(page, BehaviorJSON.Object(new System.Collections.Generic.Dictionary<string, BehaviorJSON> { ["defaultSize"] = BehaviorJSON.From(20), ["maxSize"] = BehaviorJSON.From(100), ["sort"] = BehaviorJSON.Array(mapping.Get("idField")) })));
    private static void Identifier(BehaviorJSON id) { if (id.Value.ValueKind == System.Text.Json.JsonValueKind.String) { if (id.Text.Length == 0 || System.Text.Encoding.UTF8.GetByteCount(id.Text) > 256) throw new BehaviorError("ENTITY_ID"); } else { _ = id.Integer; } }
    public System.Threading.Tasks.Task<BehaviorJSON> save(BehaviorJSON value) { Identifier(value.Get(mapping.Get("idField").Text)); value.Bound(1048576); return context.ExecuteEntity(mapping, "save", value); }
    public System.Threading.Tasks.Task<BehaviorJSON> delete(BehaviorJSON id) { Identifier(id); return context.ExecuteEntity(mapping, "delete", id); }
}

public static class BehaviorCalls {
    static readonly System.Collections.Generic.List<string> keys = new System.Collections.Generic.List<string>();
    public static void Record(string key) { lock (keys) { if (!keys.Contains(key)) keys.Add(key); } }
    public static string TakeJson() {
        lock (keys) {
            var quoted = keys.ConvertAll(key => "\"" + key + "\"");
            keys.Clear();
            return "[" + string.Join(",", quoted) + "]";
        }
    }
}
