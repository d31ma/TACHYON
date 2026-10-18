#[Entity({"adapter":"indexeddb"})]
#[Table("programNotes")]
struct ProgramNoteEntity {
    #[Id]
    id: String,
    text: String,
}
