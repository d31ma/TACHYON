@Entity({"adapter":"indexeddb"})
@Table("programNotes")
class ProgramNoteEntity {
    @Id
    var id: String = ""
    var text: String = ""
}
