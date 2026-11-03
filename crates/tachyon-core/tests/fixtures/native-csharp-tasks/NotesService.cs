[Service]
public sealed class NotesService
{
    [Needs]
    private readonly NotesClient _notesClient;

    public int Count = 0;

    public int Total()
    {
        return Count;
    }

    public async System.Threading.Tasks.Task<BehaviorJSON> Load()
    {
        Count++;
        var note = await _notesClient.first();
        Count++;
        return note;
    }

    public System.Threading.Tasks.Task<BehaviorJSON> Missing()
    {
        return _notesClient.missing();
    }

    public System.Threading.Tasks.Task<BehaviorJSON> Slow()
    {
        return _notesClient.slow();
    }

    public System.Threading.Tasks.Task<BehaviorJSON> Large()
    {
        return _notesClient.large();
    }
}
