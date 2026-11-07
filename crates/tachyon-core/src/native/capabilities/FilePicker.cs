public sealed class FilePicker
{
    private readonly NativeCapabilityOwner owner;
    internal FilePicker(NativeCapabilityOwner owner) { this.owner = owner; }

    public bool Available() => Answer("available", new()).Status == FilePickerStatus.Available;

    public FilePickerResult Open(FilePickerOptions options = null)
    {
        options ??= new();
        var types = options.Types ?? Array.Empty<string>();
        if (options.Multiple || types.Any(type => type != "text/plain" && type != "application/json"))
            return new(FilePickerStatus.Invalid);
        var values = new System.Text.Json.Nodes.JsonArray();
        foreach (var type in types) values.Add(System.Text.Json.Nodes.JsonValue.Create(type));
        return Answer("open", new() { ["types"] = values });
    }

    public FilePickerResult ReadText(string handle) => Answer("readText", new() { ["handle"] = handle });
    public FilePickerResult Release(string handle) => Answer("release", new() { ["handle"] = handle });
    private FilePickerResult Answer(string operation, System.Text.Json.Nodes.JsonObject fields) =>
        FilePickerResult.Decode(owner.Answer("filePicker." + operation, fields));
}
