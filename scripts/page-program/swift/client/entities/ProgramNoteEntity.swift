@Entity({"adapter":"indexeddb"})
@Table("programNotes")
final class ProgramNoteEntity {
    @Id
    var id: String = ""
    var text: String = ""
}
