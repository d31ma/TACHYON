[Service]
public sealed class RuntimeNoteService {
    public string Runtime = "C#, compiled by .NET";

    public bool ShowsNotes(bool development) => development;
}
