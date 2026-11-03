@Entity({"adapter":"indexeddb"})
@Table('programNotes')
class ProgramNoteEntity {
  @Id
  String id = '';
  String text = '';
}
