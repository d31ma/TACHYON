[Service]
public sealed class ReaderService
{
    private const string NoteId = "note";
    private const string TextField = "text";

    [Needs]
    private readonly PageProgramClient _pageProgramClient;

    [Needs]
    private readonly ProgramNoteStore _programNoteStore;

    public string Identity = "";
    public int Count = 0;
    public string Recalled = "";

    public async System.Threading.Tasks.Task Inspect()
    {
        Show(await _pageProgramClient.read());
    }

    public async System.Threading.Tasks.Task Increment()
    {
        Show(await _pageProgramClient.increment());
    }

    public async System.Threading.Tasks.Task Remember(string text)
    {
        var note = new System.Collections.Generic.Dictionary<string, BehaviorJSON>
        {
            ["id"] = BehaviorJSON.From(NoteId),
            [TextField] = BehaviorJSON.From(text),
        };
        await _programNoteStore.save(BehaviorJSON.Object(note));
    }

    public async System.Threading.Tasks.Task Recall()
    {
        var note = await _programNoteStore.findById(NoteId);
        Recalled = TextOf(note);
    }

    private string TextOf(BehaviorJSON note)
    {
        return note.Has(TextField) ? note.Get(TextField).Text : "";
    }

    private void Show(Models.ProgramState state)
    {
        Identity = state.Identity;
        Count = state.Count;
    }
}
