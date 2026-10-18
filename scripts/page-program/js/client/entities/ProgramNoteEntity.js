@Entity({"adapter":"indexeddb"})
@Table('programNotes')
class ProgramNoteEntity {
  @Id
  id;
  text;
}
