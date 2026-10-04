[Service]
[NeedsAll]
public class PickerService
{
    private readonly FilePicker filePicker;
    public string Status = "Ready";

    public void Choose()
    {
        var chosen = filePicker.Open(new FilePickerOptions(Types: new[] { "text/plain" }));
        Status = ReadSelected(chosen);
    }

    private string ReadSelected(FilePickerResult chosen)
    {
        if (chosen.Handle == null) return chosen.Status.ToString();
        var content = filePicker.ReadText(chosen.Handle);
        filePicker.Release(chosen.Handle);
        return content.Status.ToString();
    }
}
