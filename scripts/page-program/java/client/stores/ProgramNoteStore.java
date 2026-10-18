@Store(ProgramNoteEntity.class, {"fields":["id","text"],"defaultSize":20,"maxSize":100,"setup":"PageProgramSetup"})
interface ProgramNoteStore extends Repository<ProgramNoteEntity, String> {
}
