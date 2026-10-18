// @Entity({"adapter":"indexeddb"})
// @Table("programNotes")
type ProgramNoteEntity struct {
	// @Id
	id   string
	text string
}
