
public sealed class BrokenRule
{
    public BrokenRule(string field, string rule)
    {
        Field = field;
        Rule = rule;
    }

    public string Field { get; }

    public string Rule { get; }
}

public abstract class YonBuiltInError : System.Exception
{
    protected YonBuiltInError(string name, int status, string reference) : base(name)
    {
        Status = status;
        Reference = reference ?? "";
    }

    public int Status { get; }

    public string Reference { get; }
}

public sealed class NotSignedIn : YonBuiltInError
{
    public NotSignedIn(string reference) : base("NotSignedIn", 401, reference) {}
}

public sealed class RoleMissing : YonBuiltInError
{
    public RoleMissing(string reference) : base("RoleMissing", 403, reference) {}
}

public sealed class IdentityCheckFailed : YonBuiltInError
{
    public IdentityCheckFailed(string reference) : base("IdentityCheckFailed", 500, reference) {}
}

public sealed class RequestRejected : YonBuiltInError
{
    private readonly string message;

    public RequestRejected(string reference, string part, string field, string message) : base("RequestRejected", 400, reference)
    {
        Part = part ?? "";
        Field = field ?? "";
        this.message = message ?? "";
    }

    public string Part { get; }

    public string Field { get; }

    public override string Message => message;
}

public sealed class BodyTooLarge : YonBuiltInError
{
    public BodyTooLarge(string reference, long limitBytes) : base("BodyTooLarge", 413, reference)
    {
        LimitBytes = limitBytes;
    }

    public long LimitBytes { get; }
}

public sealed class CheckRejected : YonBuiltInError
{
    public CheckRejected(string reference, System.Collections.Generic.IReadOnlyList<BrokenRule> violations, bool truncated) : base("CheckRejected", 422, reference)
    {
        Violations = violations ?? System.Array.Empty<BrokenRule>();
        Truncated = truncated;
    }

    public System.Collections.Generic.IReadOnlyList<BrokenRule> Violations { get; }

    public bool Truncated { get; }
}

public sealed class UnexpectedError : YonBuiltInError
{
    public UnexpectedError(string reference) : base("UnexpectedError", 500, reference) {}
}

static class __YonBuiltInErrors
{
    internal static System.Exception Create(System.Text.Json.Nodes.JsonArray values)
    {
        var fields = values.Count > 0 ? values[0] as System.Text.Json.Nodes.JsonObject : null;
        if (fields is null) throw new System.InvalidOperationException("Unknown built-in error.");
        var reference = Text(fields, "reference");
        switch (Text(fields, "type"))
        {
            case "NotSignedIn": return new NotSignedIn(reference);
            case "RoleMissing": return new RoleMissing(reference);
            case "IdentityCheckFailed": return new IdentityCheckFailed(reference);
            case "RequestRejected": return new RequestRejected(reference, Text(fields, "part"), Text(fields, "field"), Text(fields, "message"));
            case "BodyTooLarge": return new BodyTooLarge(reference, Number(fields, "limitBytes"));
            case "CheckRejected": return new CheckRejected(reference, Rules(fields), Flag(fields, "truncated"));
            case "UnexpectedError": return new UnexpectedError(reference);
        }
        throw new System.InvalidOperationException("Unknown built-in error.");
    }

    static string Text(System.Text.Json.Nodes.JsonObject fields, string name) =>
        fields[name] is System.Text.Json.Nodes.JsonValue value && value.TryGetValue(out string text) ? text : "";

    static long Number(System.Text.Json.Nodes.JsonObject fields, string name) =>
        fields[name] is System.Text.Json.Nodes.JsonValue value && value.TryGetValue(out long number) ? number : 0;

    static bool Flag(System.Text.Json.Nodes.JsonObject fields, string name) =>
        fields[name] is System.Text.Json.Nodes.JsonValue value && value.TryGetValue(out bool flag) && flag;

    static System.Collections.Generic.List<BrokenRule> Rules(System.Text.Json.Nodes.JsonObject fields)
    {
        var rules = new System.Collections.Generic.List<BrokenRule>();
        if (fields["violations"] is not System.Text.Json.Nodes.JsonArray entries) return rules;
        foreach (var entry in entries)
        {
            if (entry is System.Text.Json.Nodes.JsonObject rule) rules.Add(new BrokenRule(Text(rule, "field"), Text(rule, "rule")));
        }
        return rules;
    }
}
