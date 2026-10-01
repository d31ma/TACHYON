public sealed record FilePickerResult(FilePickerStatus Status, string Handle = null, string Name = null, string Text = null)
{
    internal static FilePickerResult Decode(string text)
    {
        try
        {
            if (text == null || text.Length > 786432) return new(FilePickerStatus.Unavailable);
            var value = System.Text.Json.Nodes.JsonNode.Parse(text);
            var status = value?["status"]?.GetValue<string>();
            var parsed = status switch
            {
                "available" => FilePickerStatus.Available, "selected" => FilePickerStatus.Selected,
                "cancelled" => FilePickerStatus.Cancelled, "read" => FilePickerStatus.Read,
                "released" => FilePickerStatus.Released, "denied" => FilePickerStatus.Denied,
                "unsupported" => FilePickerStatus.Unsupported, "invalid" => FilePickerStatus.Invalid,
                "disposed" => FilePickerStatus.Disposed, "tooLarge" => FilePickerStatus.TooLarge,
                _ => FilePickerStatus.Unavailable
            };
            if (parsed == FilePickerStatus.Selected)
            {
                var handle = value?["handle"]?.GetValue<string>();
                var name = value?["name"]?.GetValue<string>();
                if (string.IsNullOrEmpty(handle) || handle.Length > 256 || handle.IndexOfAny(new[] { '/', '\\', '\0' }) >= 0 || name == null || name.Length > 512 || name.IndexOfAny(new[] { '/', '\\', '\0' }) >= 0)
                    return new(FilePickerStatus.Unavailable);
                return new(parsed, handle, name);
            }
            if (parsed == FilePickerStatus.Read)
            {
                var content = value?["text"]?.GetValue<string>();
                return content != null && System.Text.Encoding.UTF8.GetByteCount(content) <= 65536 ? new(parsed, Text: content) : new(FilePickerStatus.Unavailable);
            }
            return new(parsed);
        }
        catch (Exception) { return new(FilePickerStatus.Unavailable); }
    }
}
