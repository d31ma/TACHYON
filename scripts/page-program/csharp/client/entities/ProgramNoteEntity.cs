[Entity({"adapter":"indexeddb"})]
[Table("programNotes")]
public sealed class ProgramNoteEntity
{
    [Id]
    public string id;
    public string text;
}
