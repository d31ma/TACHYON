[<Entity({"adapter":"indexeddb"})>]
[<Table("programNotes")>]
type ProgramNoteEntity() =
    [<Id>]
    member val id : string = "" with get, set
    member val text : string = "" with get, set
