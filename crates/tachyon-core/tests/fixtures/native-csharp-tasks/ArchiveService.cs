[Service]
public sealed class ArchiveService
{
    [Needs]
    private readonly NotesClient _notesClient;

    public int Count = 0;

    public async System.Threading.Tasks.Task<BehaviorJSON> Load()
    {
        var note = await _notesClient.first();
        Count++;
        return note;
    }
}
