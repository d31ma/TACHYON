internal sealed class BehaviorHttpRevalidation
{
    private const int MaximumKeys = 4096;
    private readonly System.Collections.Generic.HashSet<string> keys = new(System.StringComparer.Ordinal);
    private bool all;

    internal BehaviorJSON With(string key, BehaviorJSON metadata) => Rewrite(metadata, Contains(key));
    internal void Mark(string key) { lock (keys) { Grow(key); } }
    internal void Forget(string key) { lock (keys) { keys.Remove(key); } }
    internal void Clear() { lock (keys) { keys.Clear(); all = false; } }

    private bool Contains(string key) { lock (keys) { return all || keys.Contains(key); } }

    private static BehaviorJSON Rewrite(BehaviorJSON metadata, bool revalidate)
    {
        var copy = new System.Collections.Generic.Dictionary<string, BehaviorJSON>(metadata.Members)
        {
            ["_httpRevalidate"] = BehaviorJSON.From(revalidate),
        };
        return BehaviorJSON.Object(copy);
    }

    private void Grow(string key)
    {
        switch (all, keys.Count >= MaximumKeys)
        {
            case (true, _): return;
            case (false, true): keys.Clear(); all = true; return;
            default: keys.Add(key); return;
        }
    }
}
