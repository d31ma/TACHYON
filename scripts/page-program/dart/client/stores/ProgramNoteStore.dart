@Store(ProgramNoteEntity, {"fields":["id","text"],"defaultSize":20,"maxSize":100,"setup":"PageProgramSetup"})
abstract interface class ProgramNoteStore implements Repository<ProgramNoteEntity, String> {
}
