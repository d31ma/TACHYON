// @Store(ProgramNoteEntity, {"fields":["id","text"],"defaultSize":20,"maxSize":100,"setup":"PageProgramSetup"})
type ProgramNoteStore interface {
	Repository[ProgramNoteEntity, string]
}
