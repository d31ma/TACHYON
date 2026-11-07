sealed class ApplicationEventPublisher {
    public T PublishEvent<T>(T value, bool retain = false) => __YonEvents.PublishModel(value, retain);
    public T publishEvent<T>(T value, bool retain = false) => PublishEvent(value, retain);
}
sealed class __YonEvents : System.IDisposable {
    private sealed class Subscription {
        internal string Name; internal System.Func<System.Text.Json.JsonElement, object> Callback; internal long Order; internal System.Text.Json.JsonElement? Condition; internal System.Text.Json.JsonElement? Models; internal string Returns; internal int Sequence; internal bool Background;
    }
    private static readonly System.Threading.AsyncLocal<__YonEvents> current = new();
    private readonly System.Collections.Generic.Queue<(string Name,string Json)> queue = new();
    private readonly System.Collections.Generic.Dictionary<string,string> retained = new();
    private readonly System.Collections.Generic.List<Subscription> listeners = new();
    internal readonly System.Collections.Generic.List<System.Action> destroyers = new();
    private readonly System.Collections.Generic.List<(string Name,string Json)> outbound = new();
    private readonly System.Collections.Generic.HashSet<string> topics;
    private System.Collections.Generic.HashSet<string> only;
    private (string Name,string Json)? delivered;
    private bool closed;
    private int deliveries;
    internal __YonEvents(params string[] topics) {
        if (current.Value != null) throw new System.InvalidOperationException("Nested local event invocation");
        this.topics = new(topics);
        current.Value = this;
    }
    internal static __YonEvents Delivery(string body, params string[] topics) {
        var events = new __YonEvents(topics);
        using var document = System.Text.Json.JsonDocument.Parse(body);
        var delivery = document.RootElement;
        events.only = new();
        foreach (var key in delivery.GetProperty("listeners").EnumerateArray()) events.only.Add(key.GetString());
        events.delivered = (delivery.GetProperty("topic").GetString(), delivery.GetProperty("payload").GetRawText());
        return events;
    }
    internal void ReceiveDelivery() { if (delivered.HasValue) queue.Enqueue(delivered.Value); }
    internal static T Publish<T>(string name, T value, bool retain) {
        try {
        var scope = current.Value;
        if (scope == null || scope.closed) throw new System.InvalidOperationException("Local event owner is disposed");
        var json = value is System.Text.Json.JsonElement element ? element.GetRawText() : System.Text.Json.JsonSerializer.Serialize(value, value.GetType(), YonModels.Options);
        if (System.Text.Encoding.UTF8.GetByteCount(json) > 65536) throw new System.InvalidOperationException("Local event capacity exceeded");
        lock (scope.queue) {
        if (scope.queue.Count >= 128) throw new System.InvalidOperationException("Local event capacity exceeded");
        if (retain) scope.retained[name] = json;
        scope.queue.Enqueue((name,json));
        if (scope.topics.Contains(name)) scope.outbound.Add((name,json));
        }
        } catch (System.Exception) { System.Console.Error.WriteLine("Yon local event publication failed."); }
        return value;
    }
    internal static T PublishModel<T>(T value, bool retain) {
        bool plain = value is null || value is string || value is bool || value is System.Enum || value is System.Text.Json.JsonElement
            || value is System.Collections.IEnumerable || value.GetType().IsPrimitive || value is decimal;
        if (plain) { System.Console.Error.WriteLine("Yon local event publication failed."); return value; }
        return Publish(value.GetType().Name, value, retain);
    }
    internal static bool Fits(System.Text.Json.JsonElement models, string shape, System.Text.Json.JsonElement value) {
        var kind = Kind(value);
        if (shape.StartsWith("?")) return kind == System.Text.Json.JsonValueKind.Null || Fits(models, shape.Substring(1), value);
        if (shape.StartsWith("[]")) {
            if (kind != System.Text.Json.JsonValueKind.Array) return false;
            foreach (var item in value.EnumerateArray()) if (!Fits(models, shape.Substring(2), item)) return false;
            return true;
        }
        switch (shape) {
            case "any": return true;
            case "string": return kind == System.Text.Json.JsonValueKind.String;
            case "integer": return kind == System.Text.Json.JsonValueKind.Number && value.TryGetInt64(out _);
            case "number": return kind == System.Text.Json.JsonValueKind.Number;
            case "boolean": return kind is System.Text.Json.JsonValueKind.True or System.Text.Json.JsonValueKind.False;
        }
        if (!models.TryGetProperty(shape, out var model) || kind != System.Text.Json.JsonValueKind.Object) return false;
        var fields = model.GetProperty("fields");
        bool open = model.TryGetProperty("open", out var declared) && declared.ValueKind == System.Text.Json.JsonValueKind.True;
        if (!open) foreach (var property in value.EnumerateObject()) if (!fields.TryGetProperty(property.Name, out _)) return false;
        foreach (var entry in fields.EnumerateObject()) {
            var field = entry.Value.GetString();
            bool required = field.StartsWith("!");
            if (!value.TryGetProperty(entry.Name, out var present)) { if (required) return false; continue; }
            if (!Fits(models, required ? field.Substring(1) : field, present)) return false;
        }
        return true;
    }
    internal static bool Matches(System.Text.Json.JsonElement matcher, System.Text.Json.JsonElement value, string eventName) {
        try { return Test(matcher, value, eventName); } catch (System.Exception) { return false; }
    }
    private static System.Text.Json.JsonValueKind Kind(System.Text.Json.JsonElement value) =>
        value.ValueKind == System.Text.Json.JsonValueKind.Undefined ? System.Text.Json.JsonValueKind.Null : value.ValueKind;
    private static System.Text.Json.JsonElement Operand(System.Text.Json.JsonElement node, System.Text.Json.JsonElement value, string eventName) {
        var kind = node.GetProperty("k").GetString();
        if (kind == "lit") return node.TryGetProperty("v", out var literal) ? literal : default;
        if (kind == "event") return System.Text.Json.JsonSerializer.SerializeToElement(eventName);
        var current = value;
        if (!node.TryGetProperty("path", out var path)) return current;
        foreach (var segment in path.EnumerateArray()) {
            if (current.ValueKind == System.Text.Json.JsonValueKind.Object) {
                var key = segment.ValueKind == System.Text.Json.JsonValueKind.String ? segment.GetString() : segment.GetRawText();
                if (!current.TryGetProperty(key, out current)) return default;
            } else if (current.ValueKind == System.Text.Json.JsonValueKind.Array && segment.ValueKind == System.Text.Json.JsonValueKind.Number && segment.TryGetInt32(out var index) && index >= 0 && index < current.GetArrayLength()) {
                current = current[index];
            } else return default;
        }
        return current;
    }
    private static bool Truthy(System.Text.Json.JsonElement value) => Kind(value) switch {
        System.Text.Json.JsonValueKind.Null => false,
        System.Text.Json.JsonValueKind.False => false,
        System.Text.Json.JsonValueKind.True => true,
        System.Text.Json.JsonValueKind.Number => value.GetDouble() != 0,
        System.Text.Json.JsonValueKind.String => value.GetString().Length != 0,
        _ => true,
    };
    private static bool Test(System.Text.Json.JsonElement node, System.Text.Json.JsonElement value, string eventName) => node.GetProperty("k").GetString() switch {
        "and" => Test(node.GetProperty("l"), value, eventName) && Test(node.GetProperty("r"), value, eventName),
        "or" => Test(node.GetProperty("l"), value, eventName) || Test(node.GetProperty("r"), value, eventName),
        "not" => !Test(node.GetProperty("e"), value, eventName),
        "truthy" => Truthy(Operand(node.GetProperty("e"), value, eventName)),
        "cmp" => Compare(node.GetProperty("op").GetString(), Operand(node.GetProperty("l"), value, eventName), Operand(node.GetProperty("r"), value, eventName)),
        _ => false,
    };
    private static bool Compare(string op, System.Text.Json.JsonElement left, System.Text.Json.JsonElement right) {
        var leftKind = Kind(left); var rightKind = Kind(right);
        bool numbers = leftKind == System.Text.Json.JsonValueKind.Number && rightKind == System.Text.Json.JsonValueKind.Number;
        bool strings = leftKind == System.Text.Json.JsonValueKind.String && rightKind == System.Text.Json.JsonValueKind.String;
        if (op == "eq" || op == "ne") {
            bool equal = numbers ? left.GetDouble() == right.GetDouble()
                : strings ? left.GetString() == right.GetString()
                : leftKind == rightKind && leftKind is System.Text.Json.JsonValueKind.True or System.Text.Json.JsonValueKind.False or System.Text.Json.JsonValueKind.Null;
            return (op == "eq") == equal;
        }
        int order;
        if (numbers) order = left.GetDouble().CompareTo(right.GetDouble());
        else if (strings) order = string.CompareOrdinal(left.GetString(), right.GetString());
        else return false;
        return op switch { "lt" => order < 0, "le" => order <= 0, "gt" => order > 0, _ => order >= 0 };
    }
    internal void Listen(string options, System.Func<System.Text.Json.JsonElement, object> callback, bool replay, string key) {
        if (only != null && !only.Contains(key)) return;
        using var document = System.Text.Json.JsonDocument.Parse(options);
        var compiled = document.RootElement;
        long order = compiled.TryGetProperty("order", out var declared) && declared.ValueKind == System.Text.Json.JsonValueKind.Number ? declared.GetInt64() : 0;
        System.Text.Json.JsonElement? condition = compiled.TryGetProperty("condition", out var matcher) && matcher.ValueKind == System.Text.Json.JsonValueKind.Object ? matcher.Clone() : null;
        System.Text.Json.JsonElement? models = compiled.TryGetProperty("models", out var declaredModels) && declaredModels.ValueKind == System.Text.Json.JsonValueKind.Object ? declaredModels.Clone() : null;
        string returns = compiled.TryGetProperty("returns", out var returned) && returned.ValueKind == System.Text.Json.JsonValueKind.String ? returned.GetString() : null;
        bool background = compiled.TryGetProperty("async", out var deferred) && deferred.ValueKind == System.Text.Json.JsonValueKind.True;
        foreach (var name in compiled.GetProperty("events").EnumerateArray()) {
            if (listeners.Count >= 1024) throw new System.InvalidOperationException("Local listener capacity exceeded");
            var eventName = name.GetString();
            listeners.Add(new Subscription { Name = eventName, Callback = callback, Order = order, Condition = condition, Models = models, Returns = returns, Sequence = listeners.Count, Background = background });
            if (replay && retained.TryGetValue(eventName,out var json)) queue.Enqueue((eventName,json));
        }
        listeners.Sort((left, right) => left.Order != right.Order ? left.Order.CompareTo(right.Order) : left.Sequence.CompareTo(right.Sequence));
    }
    private static void Republish(object result, string returns) {
        if (result == null || returns == null) return;
        var element = result is System.Text.Json.JsonElement ready ? ready : System.Text.Json.JsonSerializer.SerializeToElement(result, result.GetType(), YonModels.Options);
        var items = element.ValueKind == System.Text.Json.JsonValueKind.Array ? System.Linq.Enumerable.Take(element.EnumerateArray(), 32) : System.Linq.Enumerable.Repeat(element, 1);
        foreach (var item in items) {
            if (item.ValueKind != System.Text.Json.JsonValueKind.Null) Publish(returns, item, false);
        }
    }
    internal void Drain() {
        while (queue.Count != 0) {
            var item = queue.Dequeue();
            if (++deliveries > 4096) { queue.Clear(); System.Console.Error.WriteLine("Yon local event delivery bound exceeded."); return; }
            var deferred = new System.Collections.Generic.List<(Subscription Listener, System.Text.Json.JsonElement Value)>();
            foreach (var listener in listeners) if (listener.Name == item.Name) {
                try {
                    using var document = System.Text.Json.JsonDocument.Parse(item.Json);
                    var value = document.RootElement.Clone();
                    if (listener.Models is System.Text.Json.JsonElement models && !Fits(models, item.Name, value)) { System.Console.Error.WriteLine("Yon local event payload does not fit the listener model."); continue; }
                    if (listener.Condition is System.Text.Json.JsonElement condition && !Matches(condition, value, item.Name)) continue;
                    if (listener.Background) { deferred.Add((listener, value)); continue; }
                    Republish(listener.Callback(value), listener.Returns);
                }
                catch (System.Exception) { System.Console.Error.WriteLine("Yon local event listener failed."); }
            }
            foreach (var (listener, value) in deferred) {
                try { if (listener.Callback(value) is System.Threading.Tasks.Task task) task.GetAwaiter().GetResult(); }
                catch (System.Exception) { System.Console.Error.WriteLine("Yon local event listener failed."); }
            }
        }
        SendTopics();
    }
    private void SendTopics() {
        foreach (var item in outbound) {
            try { YonManagedResources.Call(new System.Collections.Generic.Dictionary<string, string> { ["operation"] = "topic", ["topic"] = item.Name, ["payload_json"] = item.Json }); }
            catch (System.Exception) { System.Console.Error.WriteLine("Yon topic publication failed."); }
        }
        outbound.Clear();
    }
    public void Dispose() {
        if (closed) return; closed = true; queue.Clear(); listeners.Clear();
        for (var index = destroyers.Count-1; index >= 0; index--) {
            try { destroyers[index](); } catch (System.Exception) { System.Console.Error.WriteLine("Yon local lifecycle cleanup failed."); }
        }
        retained.Clear(); current.Value = null;
    }
}
