[Service]
[NeedsAll]
public class LanguagesService
{
    public required object Entries;

    public object Handlers()
    {
        return FilesOf(ListValue(Entries));
    }

    private static List<object> FilesOf(object[] entries)
    {
        return entries.SelectMany(EntryFiles).ToList();
    }

    private static object[] EntryFiles(object entry)
    {
        return ListValue(Value(MapValue(entry), "files"));
    }

    private static IDictionary<string, object> MapValue(object value)
    {
        return value as IDictionary<string, object> ?? new Dictionary<string, object>();
    }

    private static object Value(IDictionary<string, object> map, string key)
    {
        return map.TryGetValue(key, out var value) ? value : null;
    }

    private static object[] ListValue(object value)
    {
        return value as object[] ?? System.Array.Empty<object>();
    }
}
